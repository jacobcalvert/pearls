use sea_orm_migration::prelude::ConnectionTrait;
use sea_orm_migration::sea_orm::{
    DatabaseConnection, DbBackend, DbErr, ExecResult, QueryResult, Statement,
};
use sea_query::{Expr, Iden, InsertStatement, OnConflict, Query, SimpleExpr, SqliteQueryBuilder};
use serde::Serialize;
use std::collections::{HashMap, HashSet};

use crate::cli::TaskState;

#[derive(Iden)]
enum Task {
    Table,
    Id,
    Title,
    Desc,
    Priority,
    State,
    Assignee,
}

#[derive(Iden)]
enum Dependency {
    Table,
    ParentId,
    ChildId,
}

pub async fn add_task(
    conn: &DatabaseConnection,
    title: &str,
    description: &str,
    priority: Option<i64>,
    assignee: Option<&str>,
) -> Result<TaskRow, DbErr> {
    let mut insert = InsertStatement::new();
    let mut columns: Vec<Task> = vec![Task::Title, Task::Desc];
    let mut values: Vec<SimpleExpr> = vec![Expr::val(title).into(), Expr::val(description).into()];
    if let Some(priority) = priority {
        columns.push(Task::Priority);
        values.push(Expr::val(priority).into());
    }
    if let Some(assignee) = assignee {
        columns.push(Task::Assignee);
        values.push(Expr::val(assignee).into());
    }
    insert
        .into_table(Task::Table)
        .columns(columns)
        .values(values)
        .map_err(|err| DbErr::Custom(err.to_string()))?;

    let (sql, values) = insert.build(SqliteQueryBuilder);
    conn.execute(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        sql,
        values,
    ))
    .await?;

    let (sql, values) = Query::select()
        .expr(Expr::cust("last_insert_rowid()"))
        .build(SqliteQueryBuilder);
    let row: QueryResult = conn
        .query_one(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            sql,
            values,
        ))
        .await?
        .ok_or_else(|| DbErr::Custom("failed to read last_insert_rowid".to_string()))?;
    let id: i64 = row.try_get_by_index(0)?;
    get_task_by_id(conn, id).await
}

pub async fn get_task_by_id(conn: &DatabaseConnection, id: i64) -> Result<TaskRow, DbErr> {
    let (sql, values) = Query::select()
        .columns([
            (Task::Table, Task::Id),
            (Task::Table, Task::Title),
            (Task::Table, Task::Desc),
            (Task::Table, Task::Priority),
            (Task::Table, Task::State),
            (Task::Table, Task::Assignee),
        ])
        .from(Task::Table)
        .and_where(Expr::col((Task::Table, Task::Id)).eq(id))
        .build(SqliteQueryBuilder);

    let row = conn
        .query_one(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            sql,
            values,
        ))
        .await?
        .ok_or_else(|| DbErr::Custom(format!("task {id} not found")))?;

    let id: i64 = row.try_get_by_index(0)?;
    let mut task = TaskRow {
        id,
        title: row.try_get_by_index(1)?,
        desc: row.try_get_by_index(2)?,
        priority: row.try_get_by_index(3)?,
        state: row.try_get_by_index(4)?,
        assignee: row.try_get_by_index(5)?,
        parents: Vec::new(),
        children: Vec::new(),
    };

    populate_dependencies(
        conn,
        std::slice::from_ref(&id),
        std::slice::from_mut(&mut task),
    )
    .await?;
    Ok(task)
}

pub async fn list_tasks(
    conn: &DatabaseConnection,
    states: &[TaskState],
) -> Result<Vec<TaskRow>, DbErr> {
    let mut query = Query::select();
    query
        .columns([
            (Task::Table, Task::Id),
            (Task::Table, Task::Title),
            (Task::Table, Task::Desc),
            (Task::Table, Task::Priority),
            (Task::Table, Task::State),
            (Task::Table, Task::Assignee),
        ])
        .from(Task::Table);

    let (sql, values) = query.build(SqliteQueryBuilder);
    let rows: Vec<QueryResult> = conn
        .query_all(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            sql,
            values,
        ))
        .await?;

    let mut tasks = Vec::with_capacity(rows.len());
    for row in rows {
        tasks.push(TaskRow {
            id: row.try_get_by_index(0)?,
            title: row.try_get_by_index(1)?,
            desc: row.try_get_by_index(2)?,
            priority: row.try_get_by_index(3)?,
            state: row.try_get_by_index(4)?,
            assignee: row.try_get_by_index(5)?,
            parents: Vec::new(),
            children: Vec::new(),
        });
    }

    let ids: Vec<i64> = tasks.iter().map(|task| task.id).collect();
    populate_dependencies(conn, &ids, &mut tasks).await?;

    Ok(filter_tasks_by_states(tasks, states))
}

pub async fn list_tasks_paginated(
    conn: &DatabaseConnection,
    states: &[TaskState],
    offset: u64,
    limit: u64,
) -> Result<Vec<TaskRow>, DbErr> {
    let mut query = Query::select();
    query
        .columns([
            (Task::Table, Task::Id),
            (Task::Table, Task::Title),
            (Task::Table, Task::Desc),
            (Task::Table, Task::Priority),
            (Task::Table, Task::State),
            (Task::Table, Task::Assignee),
        ])
        .from(Task::Table);

    let (sql, values) = query.build(SqliteQueryBuilder);
    let rows: Vec<QueryResult> = conn
        .query_all(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            sql,
            values,
        ))
        .await?;

    let mut tasks = Vec::with_capacity(rows.len());
    for row in rows {
        tasks.push(TaskRow {
            id: row.try_get_by_index(0)?,
            title: row.try_get_by_index(1)?,
            desc: row.try_get_by_index(2)?,
            priority: row.try_get_by_index(3)?,
            state: row.try_get_by_index(4)?,
            assignee: row.try_get_by_index(5)?,
            parents: Vec::new(),
            children: Vec::new(),
        });
    }

    let ids: Vec<i64> = tasks.iter().map(|task| task.id).collect();
    populate_dependencies(conn, &ids, &mut tasks).await?;
    let filtered = filter_tasks_by_states(tasks, states);

    let offset = offset.min(usize::MAX as u64) as usize;
    let limit = limit.min(usize::MAX as u64) as usize;

    Ok(filtered.into_iter().skip(offset).take(limit).collect())
}

pub async fn claim_next(
    conn: &DatabaseConnection,
    assignee: Option<&str>,
) -> Result<Option<TaskRow>, DbErr> {
    let mut ready = list_tasks(conn, &[TaskState::Ready]).await?;
    if ready.is_empty() {
        return Ok(None);
    }

    ready.sort_by_key(|task| (task.priority, task.id));
    let next_id = ready[0].id;

    update_metadata(
        conn,
        next_id,
        None,
        None,
        None,
        Some(TaskState::InProgress),
        assignee,
        false,
    )
    .await?;
    let updated = get_task_by_id(conn, next_id).await?;
    Ok(Some(updated))
}

#[allow(clippy::too_many_arguments)]
pub async fn update_metadata(
    conn: &DatabaseConnection,
    id: i64,
    title: Option<&str>,
    desc: Option<&str>,
    priority: Option<i64>,
    state: Option<TaskState>,
    assignee: Option<&str>,
    no_assignee: bool,
) -> Result<u64, DbErr> {
    let mut update = sea_query::UpdateStatement::new();
    update
        .table(Task::Table)
        .and_where(Expr::col(Task::Id).eq(id));

    let mut changes = 0;
    if let Some(title) = title {
        update.value(Task::Title, title);
        changes += 1;
    }
    if let Some(desc) = desc {
        update.value(Task::Desc, desc);
        changes += 1;
    }
    if let Some(priority) = priority {
        update.value(Task::Priority, priority);
        changes += 1;
    }
    if let Some(state) = state {
        update.value(Task::State, state.as_str());
        changes += 1;
    }
    if let Some(assignee) = assignee {
        update.value(Task::Assignee, assignee);
        changes += 1;
    }
    if no_assignee {
        update.value(Task::Assignee, Option::<String>::None);
        changes += 1;
    }

    if changes == 0 {
        return Ok(0);
    }

    let (sql, values) = update.build(SqliteQueryBuilder);
    let result: ExecResult = conn
        .execute(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            sql,
            values,
        ))
        .await?;
    Ok(result.rows_affected())
}

pub async fn add_dependency(
    conn: &DatabaseConnection,
    parent_id: i64,
    child_id: i64,
) -> Result<(), DbErr> {
    insert_dependency(conn, parent_id, child_id).await
}

pub async fn update_dependency(
    conn: &DatabaseConnection,
    id: i64,
    add_child: &[i64],
    remove_child: &[i64],
) -> Result<(), DbErr> {
    for child in add_child {
        insert_dependency(conn, id, *child).await?;
    }
    for child in remove_child {
        delete_dependency(conn, id, *child).await?;
    }

    Ok(())
}

async fn insert_dependency(
    conn: &DatabaseConnection,
    parent_id: i64,
    child_id: i64,
) -> Result<(), DbErr> {
    let mut insert = InsertStatement::new();
    insert
        .into_table(Dependency::Table)
        .columns([Dependency::ParentId, Dependency::ChildId])
        .values([Expr::val(parent_id).into(), Expr::val(child_id).into()])
        .map_err(|err| DbErr::Custom(err.to_string()))?
        .on_conflict(
            OnConflict::columns([Dependency::ParentId, Dependency::ChildId])
                .do_nothing()
                .to_owned(),
        );

    let (sql, values) = insert.build(SqliteQueryBuilder);
    conn.execute(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        sql,
        values,
    ))
    .await?;
    Ok(())
}

async fn delete_dependency(
    conn: &DatabaseConnection,
    parent_id: i64,
    child_id: i64,
) -> Result<(), DbErr> {
    let (sql, values) = Query::delete()
        .from_table(Dependency::Table)
        .and_where(Expr::col(Dependency::ParentId).eq(parent_id))
        .and_where(Expr::col(Dependency::ChildId).eq(child_id))
        .build(SqliteQueryBuilder);

    conn.execute(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        sql,
        values,
    ))
    .await?;
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskRow {
    pub id: i64,
    pub title: Option<String>,
    pub desc: Option<String>,
    pub priority: i64,
    pub state: String,
    pub assignee: Option<String>,
    pub parents: Vec<i64>,
    pub children: Vec<i64>,
}

impl TaskRow {
    pub fn display_line(&self) -> String {
        let title = self.title.as_deref().unwrap_or("");
        let desc = self.desc.as_deref().unwrap_or("");
        let assignee = self.assignee.as_deref().unwrap_or("no assignee");
        let parents = format_ids(&self.parents);
        let children = format_ids(&self.children);
        format!(
            "#{id} [{state}] p{priority} {title} - {desc} assignee={assignee} parents={parents} children={children}",
            id = self.id,
            state = self.state,
            priority = self.priority,
            title = title,
            desc = desc,
            assignee = assignee,
            parents = parents,
            children = children
        )
    }
}

impl TaskState {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskState::Ready => "ready",
            TaskState::Blocked => "blocked",
            TaskState::InProgress => "in_progress",
            TaskState::Closed => "closed",
        }
    }
}

fn format_ids(values: &[i64]) -> String {
    if values.is_empty() {
        "[]".to_string()
    } else {
        let joined = values
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(",");
        format!("[{joined}]")
    }
}

fn filter_tasks_by_states(tasks: Vec<TaskRow>, states: &[TaskState]) -> Vec<TaskRow> {
    if states.is_empty() {
        return tasks;
    }

    let allowed: HashSet<&'static str> = states.iter().map(TaskState::as_str).collect();
    tasks
        .into_iter()
        .filter(|task| allowed.contains(task.state.as_str()))
        .collect()
}

async fn populate_dependencies(
    conn: &DatabaseConnection,
    ids: &[i64],
    tasks: &mut [TaskRow],
) -> Result<(), DbErr> {
    if ids.is_empty() {
        return Ok(());
    }

    let mut parents_by_child: HashMap<i64, Vec<i64>> = HashMap::new();
    let mut children_by_parent: HashMap<i64, Vec<i64>> = HashMap::new();

    let id_exprs: Vec<SimpleExpr> = ids.iter().map(|id| Expr::val(*id).into()).collect();
    let (sql, values) = Query::select()
        .columns([Dependency::ParentId, Dependency::ChildId])
        .from(Dependency::Table)
        .and_where(
            Expr::col(Dependency::ChildId)
                .is_in(id_exprs.clone())
                .or(Expr::col(Dependency::ParentId).is_in(id_exprs)),
        )
        .build(SqliteQueryBuilder);

    let rows: Vec<QueryResult> = conn
        .query_all(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            sql,
            values,
        ))
        .await?;

    let mut parent_ids: HashSet<i64> = HashSet::new();
    for row in rows {
        let parent_id: i64 = row.try_get_by_index(0)?;
        let child_id: i64 = row.try_get_by_index(1)?;

        parents_by_child
            .entry(child_id)
            .or_default()
            .push(parent_id);
        children_by_parent
            .entry(parent_id)
            .or_default()
            .push(child_id);
        parent_ids.insert(parent_id);
    }

    let parent_state_map =
        fetch_task_states(conn, &parent_ids.into_iter().collect::<Vec<_>>()).await?;

    for task in tasks.iter_mut() {
        if let Some(parents) = parents_by_child.get(&task.id) {
            task.parents = parents.clone();
            task.parents.sort_unstable();
        }
        if let Some(children) = children_by_parent.get(&task.id) {
            task.children = children.clone();
            task.children.sort_unstable();
        }

        if task.state != "closed" {
            let blocked = task.parents.iter().any(|parent_id| {
                parent_state_map
                    .get(parent_id)
                    .is_some_and(|state| state != "closed")
            });
            if blocked {
                task.state = "blocked".to_string();
            }
        }
    }

    Ok(())
}

async fn fetch_task_states(
    conn: &DatabaseConnection,
    ids: &[i64],
) -> Result<HashMap<i64, String>, DbErr> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }

    let id_exprs: Vec<SimpleExpr> = ids.iter().map(|id| Expr::val(*id).into()).collect();
    let (sql, values) = Query::select()
        .columns([(Task::Table, Task::Id), (Task::Table, Task::State)])
        .from(Task::Table)
        .and_where(Expr::col((Task::Table, Task::Id)).is_in(id_exprs))
        .build(SqliteQueryBuilder);

    let rows: Vec<QueryResult> = conn
        .query_all(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            sql,
            values,
        ))
        .await?;

    let mut map = HashMap::new();
    for row in rows {
        let id: i64 = row.try_get_by_index(0)?;
        let state: String = row.try_get_by_index(1)?;
        map.insert(id, state);
    }

    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::conn;

    fn find_task(tasks: &[TaskRow], id: i64) -> &TaskRow {
        tasks
            .iter()
            .find(|task| task.id == id)
            .unwrap_or_else(|| panic!("missing task {id}"))
    }

    #[tokio::test(flavor = "current_thread")]
    async fn list_tasks_filters_states() {
        let temp = tempfile::tempdir().expect("tempdir");
        let db_path = temp.path().join("pearls.db");
        let conn = conn::connect(&db_path).await.expect("connect");

        let t1 = add_task(&conn, "one", "first", None, None)
            .await
            .expect("add t1");
        let t2 = add_task(&conn, "two", "second", None, None)
            .await
            .expect("add t2");
        update_metadata(
            &conn,
            t2.id,
            None,
            None,
            None,
            Some(TaskState::InProgress),
            None,
            false,
        )
        .await
        .expect("update state");

        let ready = list_tasks(&conn, &[TaskState::Ready])
            .await
            .expect("list ready");
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, t1.id);

        let all = list_tasks(&conn, &[]).await.expect("list all");
        assert_eq!(all.len(), 2);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn list_tasks_paginated_excludes_dependency_blocked_rows_from_ready_filter() {
        let temp = tempfile::tempdir().expect("tempdir");
        let db_path = temp.path().join("pearls.db");
        let conn = conn::connect(&db_path).await.expect("connect");

        let parent = add_task(&conn, "parent", "p", None, None)
            .await
            .expect("add parent");
        let child = add_task(&conn, "child", "c", None, None)
            .await
            .expect("add child");

        add_dependency(&conn, parent.id, child.id)
            .await
            .expect("add dependency");

        let ready = list_tasks_paginated(&conn, &[TaskState::Ready], 0, 20)
            .await
            .expect("list ready");
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, parent.id);
        assert_eq!(ready[0].state, "ready");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn dependency_blocks_child_until_parent_closed() {
        let temp = tempfile::tempdir().expect("tempdir");
        let db_path = temp.path().join("pearls.db");
        let conn = conn::connect(&db_path).await.expect("connect");

        let parent = add_task(&conn, "parent", "p", None, None)
            .await
            .expect("add parent");
        let child = add_task(&conn, "child", "c", None, None)
            .await
            .expect("add child");

        add_dependency(&conn, parent.id, child.id)
            .await
            .expect("add dependency");

        let tasks = list_tasks(&conn, &[]).await.expect("list");
        let child_row = find_task(&tasks, child.id);
        assert_eq!(child_row.state, "blocked");

        update_metadata(
            &conn,
            parent.id,
            None,
            None,
            None,
            Some(TaskState::Closed),
            None,
            false,
        )
        .await
        .expect("close parent");

        let tasks = list_tasks(&conn, &[]).await.expect("list");
        let child_row = find_task(&tasks, child.id);
        assert_eq!(child_row.state, "ready");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn claim_next_picks_highest_priority_ready() {
        let temp = tempfile::tempdir().expect("tempdir");
        let db_path = temp.path().join("pearls.db");
        let conn = conn::connect(&db_path).await.expect("connect");

        let low = add_task(&conn, "low", "p2", Some(2), None)
            .await
            .expect("add low");
        let high = add_task(&conn, "high", "p1", Some(1), None)
            .await
            .expect("add high");
        let tied = add_task(&conn, "tied", "p1", Some(1), None)
            .await
            .expect("add tied");

        let first = claim_next(&conn, None).await.expect("claim first");
        assert_eq!(first.as_ref().map(|task| task.id), Some(high.id));
        assert_eq!(
            first.as_ref().map(|task| task.state.as_str()),
            Some("in_progress")
        );

        let second = claim_next(&conn, None).await.expect("claim second");
        assert_eq!(second.as_ref().map(|task| task.id), Some(tied.id));

        let third = claim_next(&conn, None).await.expect("claim third");
        assert_eq!(third.as_ref().map(|task| task.id), Some(low.id));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn claim_next_sets_assignee() {
        let temp = tempfile::tempdir().expect("tempdir");
        let db_path = temp.path().join("pearls.db");
        let conn = conn::connect(&db_path).await.expect("connect");

        add_task(&conn, "task", "desc", None, None)
            .await
            .expect("add task");

        let claimed = claim_next(&conn, Some("alice")).await.expect("claim next");
        assert_eq!(
            claimed.and_then(|task| task.assignee),
            Some("alice".to_string())
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn update_metadata_can_set_and_clear_assignee() {
        let temp = tempfile::tempdir().expect("tempdir");
        let db_path = temp.path().join("pearls.db");
        let conn = conn::connect(&db_path).await.expect("connect");

        let task = add_task(&conn, "task", "desc", None, Some("alice"))
            .await
            .expect("add task");
        assert_eq!(task.assignee.as_deref(), Some("alice"));

        update_metadata(&conn, task.id, None, None, None, None, Some("bob"), false)
            .await
            .expect("assign task");
        let updated = get_task_by_id(&conn, task.id).await.expect("load updated");
        assert_eq!(updated.assignee.as_deref(), Some("bob"));

        update_metadata(&conn, task.id, None, None, None, None, None, true)
            .await
            .expect("clear assignee");
        let cleared = get_task_by_id(&conn, task.id).await.expect("load cleared");
        assert_eq!(cleared.assignee, None);
    }
}
