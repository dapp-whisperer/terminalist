use crate::entities::{project, task};
use crate::repositories::{ProjectRepository, SectionRepository, TaskRepository};
use crate::sync::SyncService;
use crate::utils::datetime;
use anyhow::Result;
use sea_orm::{ActiveValue, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, TransactionTrait};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectUpdateIntent {
    Unchanged,
    Set(Uuid),
    MoveToInbox,
}

impl SyncService {
    fn apply_backend_due_fields(active_model: &mut task::ActiveModel, backend_task: &crate::backend::BackendTask) {
        active_model.due_date = ActiveValue::Set(backend_task.due_date.clone());
        active_model.due_datetime = ActiveValue::Set(backend_task.due_datetime.clone());
        active_model.is_recurring = ActiveValue::Set(backend_task.is_recurring);
        active_model.deadline = ActiveValue::Set(backend_task.deadline.clone());
    }

    /// Retrieves all tasks for a specific project from local storage.
    ///
    /// # Arguments
    /// * `project_id` - The unique identifier of the project
    ///
    /// # Returns
    /// A vector of `task::Model` objects for the specified project
    ///
    /// # Errors
    /// Returns an error if local storage access fails
    pub async fn get_tasks_for_project(&self, project_id: &Uuid) -> Result<Vec<task::Model>> {
        let storage = self.storage.lock().await;
        TaskRepository::get_for_project(&storage.conn, project_id).await
    }

    /// Retrieves all tasks from local storage across all projects.
    ///
    /// This method is primarily used for search functionality and global task operations.
    /// It provides fast access to the complete task dataset.
    ///
    /// # Returns
    /// A vector of all `task::Model` objects in the local database
    ///
    /// # Errors
    /// Returns an error if local storage access fails
    pub async fn get_all_tasks(&self) -> Result<Vec<task::Model>> {
        let storage = self.storage.lock().await;
        TaskRepository::get_all(&storage.conn).await
    }

    /// Searches for tasks by content using database-level filtering.
    ///
    /// This method performs fast text search across task content using SQL LIKE queries.
    /// The search is case-insensitive and matches partial content.
    ///
    /// # Arguments
    /// * `query` - The search term to look for in task content
    ///
    /// # Returns
    /// A vector of `task::Model` objects matching the search criteria
    ///
    /// # Errors
    /// Returns an error if local storage access fails
    pub async fn search_tasks(&self, query: &str) -> Result<Vec<task::Model>> {
        let storage = self.storage.lock().await;
        TaskRepository::search(&storage.conn, query).await
    }

    /// Get tasks with a specific label from local storage (fast)
    pub async fn get_tasks_with_label(&self, label_id: Uuid) -> Result<Vec<task::Model>> {
        let storage = self.storage.lock().await;
        TaskRepository::get_with_label(&storage.conn, label_id).await
    }

    /// Retrieves tasks for the "Today" view with business logic.
    ///
    /// This method implements the UI business logic for the Today view by combining
    /// overdue tasks with tasks due today. Overdue tasks are shown first, followed
    /// by today's tasks.
    ///
    /// # Returns
    /// A vector of `task::Model` objects for the Today view, with overdue tasks first
    ///
    /// # Errors
    /// Returns an error if local storage access fails
    pub async fn get_tasks_for_today(&self) -> Result<Vec<task::Model>> {
        let storage = self.storage.lock().await;
        let today = datetime::format_today();
        TaskRepository::get_for_today(&storage.conn, &today).await
    }

    /// Retrieves tasks scheduled for tomorrow.
    ///
    /// This method returns only tasks that are specifically due tomorrow,
    /// without any additional business logic.
    ///
    /// # Returns
    /// A vector of `task::Model` objects due tomorrow
    ///
    /// # Errors
    /// Returns an error if local storage access fails
    pub async fn get_tasks_for_tomorrow(&self) -> Result<Vec<task::Model>> {
        let storage = self.storage.lock().await;
        let tomorrow = datetime::format_date_with_offset(1);
        TaskRepository::get_for_tomorrow(&storage.conn, &tomorrow).await
    }

    /// Retrieves tasks for the "Upcoming" view with business logic.
    ///
    /// This method implements the UI business logic for the Upcoming view by combining
    /// overdue tasks, today's tasks, and tasks due within the next 3 months.
    /// Tasks are ordered as: overdue → today → future (next 3 months).
    ///
    /// # Returns
    /// A vector of `task::Model` objects for the Upcoming view, properly ordered
    ///
    /// # Errors
    /// Returns an error if local storage access fails
    pub async fn get_tasks_for_upcoming(&self) -> Result<Vec<task::Model>> {
        let storage = self.storage.lock().await;
        let today = datetime::format_today();
        let three_months_later = datetime::format_date_with_offset(90);
        TaskRepository::get_for_upcoming(&storage.conn, &today, &three_months_later).await
    }

    /// Get a single task by ID from local storage (fast)
    pub async fn get_task_by_id(&self, task_id: &Uuid) -> Result<Option<task::Model>> {
        let storage = self.storage.lock().await;
        TaskRepository::get_by_id(&storage.conn, task_id).await
    }

    /// Creates a new task via the remote backend and stores it locally.
    ///
    /// This method creates a task remotely and immediately stores it in local storage
    /// for instant UI updates. The task will be available in the UI without requiring
    /// a full sync operation.
    ///
    /// # Arguments
    /// * `content` - The content/description of the new task
    /// * `description` - Optional task description
    /// * `due_string` - Optional natural language due date string
    /// * `project_uuid` - Optional local project UUID to assign the task to a specific project
    ///
    /// # Errors
    /// Returns an error if the backend call fails or local storage update fails
    pub async fn create_task(
        &self,
        content: &str,
        description: Option<&str>,
        due_string: Option<&str>,
        project_uuid: Option<Uuid>,
    ) -> Result<()> {
        // Look up remote_id for project if provided
        let remote_project_id = {
            let storage = self.storage.lock().await;
            if let Some(uuid) = project_uuid {
                Some(ProjectRepository::get_remote_id(&storage.conn, &uuid).await?)
            } else {
                None
            }
            // Lock is automatically dropped here when storage goes out of scope
        };

        // Create task via backend using backend CreateTaskArgs (lock is not held)
        let task_args = crate::backend::CreateTaskArgs {
            content: content.to_string(),
            description: description.map(std::string::ToString::to_string),
            project_remote_id: remote_project_id.unwrap_or_default(),
            section_remote_id: None,
            parent_remote_id: None,
            priority: None,
            due_date: None,
            due_datetime: None,
            due_string: due_string.map(std::string::ToString::to_string),
            duration: None,
            labels: Vec::new(),
        };
        let backend_task = self
            .get_backend()
            .await?
            .create_task(task_args)
            .await
            .map_err(|e| anyhow::anyhow!("Backend error: {}", e))?;

        // Store the created task in local database immediately for UI refresh
        let storage = self.storage.lock().await;
        let txn = storage.conn.begin().await?;

        // Look up local project UUID from remote project_id
        let project_uuid = Self::lookup_project_uuid(
            &txn,
            &self.backend_uuid,
            &backend_task.project_remote_id,
            "task creation",
        )
        .await?;

        // Look up local section UUID from remote section_id if present
        let section_uuid =
            Self::lookup_section_uuid(&txn, &self.backend_uuid, backend_task.section_remote_id.as_ref()).await?;

        // Look up local parent UUID from remote parent_id if present
        let parent_uuid = if let Some(remote_parent_id) = &backend_task.parent_remote_id {
            TaskRepository::get_by_remote_id(&txn, &self.backend_uuid, remote_parent_id)
                .await?
                .map(|t| t.uuid)
        } else {
            None
        };

        let local_task = task::ActiveModel {
            uuid: ActiveValue::Set(Uuid::new_v4()),
            backend_uuid: ActiveValue::Set(self.backend_uuid),
            remote_id: ActiveValue::Set(backend_task.remote_id),
            content: ActiveValue::Set(backend_task.content),
            description: ActiveValue::Set(backend_task.description),
            project_uuid: ActiveValue::Set(project_uuid),
            section_uuid: ActiveValue::Set(section_uuid),
            parent_uuid: ActiveValue::Set(parent_uuid),
            priority: ActiveValue::Set(backend_task.priority),
            order_index: ActiveValue::Set(backend_task.order_index),
            due_date: ActiveValue::Set(backend_task.due_date),
            due_datetime: ActiveValue::Set(backend_task.due_datetime),
            is_recurring: ActiveValue::Set(backend_task.is_recurring),
            deadline: ActiveValue::Set(backend_task.deadline),
            duration: ActiveValue::Set(backend_task.duration),
            is_completed: ActiveValue::Set(backend_task.is_completed),
            is_deleted: ActiveValue::Set(false),
        };

        use sea_orm::sea_query::OnConflict;
        let mut insert = task::Entity::insert(local_task);
        insert = insert.on_conflict(
            OnConflict::columns([task::Column::BackendUuid, task::Column::RemoteId])
                .update_columns([
                    task::Column::Content,
                    task::Column::Description,
                    task::Column::ProjectUuid,
                    task::Column::SectionUuid,
                    task::Column::ParentUuid,
                    task::Column::Priority,
                    task::Column::OrderIndex,
                    task::Column::DueDate,
                    task::Column::DueDatetime,
                    task::Column::IsRecurring,
                    task::Column::Deadline,
                    task::Column::Duration,
                    task::Column::IsCompleted,
                    task::Column::IsDeleted,
                ])
                .to_owned(),
        );
        insert.exec(&txn).await?;

        txn.commit().await?;

        Ok(())
    }

    /// Update task content, description, due date, and project in one backend call.
    pub async fn update_task_full(
        &self,
        task_uuid: &Uuid,
        content: &str,
        description: Option<&str>,
        due_string: Option<&str>,
        project_update: ProjectUpdateIntent,
    ) -> Result<()> {
        let remote_id = self.get_task_remote_id(task_uuid).await?;

        let project_remote_id = {
            let storage = self.storage.lock().await;
            match project_update {
                ProjectUpdateIntent::Unchanged => None,
                ProjectUpdateIntent::Set(uuid) => Some(ProjectRepository::get_remote_id(&storage.conn, &uuid).await?),
                ProjectUpdateIntent::MoveToInbox => {
                    let inbox_project = project::Entity::find()
                        .filter(project::Column::BackendUuid.eq(self.backend_uuid))
                        .filter(project::Column::IsInboxProject.eq(true))
                        .one(&storage.conn)
                        .await?;
                    inbox_project.map(|project| project.remote_id)
                }
            }
        };

        let task_args = crate::backend::UpdateTaskArgs {
            content: Some(content.to_string()),
            description: description.map(std::string::ToString::to_string),
            project_remote_id,
            section_remote_id: None,
            parent_remote_id: None,
            priority: None,
            due_date: None,
            due_datetime: None,
            due_string: due_string.map(std::string::ToString::to_string),
            duration: None,
            labels: None,
        };

        let backend_task = self
            .get_backend()
            .await?
            .update_task(&remote_id, task_args)
            .await
            .map_err(|e| anyhow::anyhow!("Backend error: {}", e))?;

        let storage = self.storage.lock().await;

        if let Some(task) = TaskRepository::get_by_id(&storage.conn, task_uuid).await? {
            let mut active_model: task::ActiveModel = task.into_active_model();
            Self::apply_backend_due_fields(&mut active_model, &backend_task);
            active_model.content = ActiveValue::Set(backend_task.content);
            active_model.description = ActiveValue::Set(backend_task.description);

            let project_model =
                ProjectRepository::get_by_remote_id(&storage.conn, &self.backend_uuid, &backend_task.project_remote_id)
                    .await?
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                    "Project with remote_id {} not found locally during task update. Please sync projects first.",
                    backend_task.project_remote_id
                )
                    })?;
            active_model.project_uuid = ActiveValue::Set(project_model.uuid);

            TaskRepository::update(&storage.conn, active_model).await?;
        }

        Ok(())
    }

    /// Update task due date
    pub async fn update_task_due_date(&self, task_uuid: &Uuid, due_date: Option<&str>) -> Result<()> {
        // Look up the task's remote_id for backend call
        let remote_id = self.get_task_remote_id(task_uuid).await?;

        // Update task via backend using the UpdateTaskArgs structure
        let task_args = crate::backend::UpdateTaskArgs {
            content: None,
            description: None,
            project_remote_id: None,
            section_remote_id: None,
            parent_remote_id: None,
            priority: None,
            due_date: due_date.map(std::string::ToString::to_string),
            due_datetime: None,
            due_string: None,
            duration: None,
            labels: None,
        };
        let _task = self
            .get_backend()
            .await?
            .update_task(&remote_id, task_args)
            .await
            .map_err(|e| anyhow::anyhow!("Backend error: {}", e))?;

        // Then update local storage
        let storage = self.storage.lock().await;

        if let Some(task) = TaskRepository::get_by_id(&storage.conn, task_uuid).await? {
            let mut active_model: task::ActiveModel = task.into_active_model();
            active_model.due_date = ActiveValue::Set(due_date.map(|s| s.to_string()));
            TaskRepository::update(&storage.conn, active_model).await?;
        }

        Ok(())
    }

    /// Update task due date using a natural language string via Todoist's due_string API.
    /// The API parses the string and returns the resolved date, which is used to update local storage.
    pub async fn update_task_due_string(&self, task_uuid: &Uuid, due_string: &str) -> Result<()> {
        let remote_id = self.get_task_remote_id(task_uuid).await?;

        let task_args = crate::backend::UpdateTaskArgs {
            content: None,
            description: None,
            project_remote_id: None,
            section_remote_id: None,
            parent_remote_id: None,
            priority: None,
            due_date: None,
            due_datetime: None,
            due_string: Some(due_string.to_string()),
            duration: None,
            labels: None,
        };
        let backend_task = self
            .get_backend()
            .await?
            .update_task(&remote_id, task_args)
            .await
            .map_err(|e| anyhow::anyhow!("Backend error: {}", e))?;

        // Update local storage from the API response (resolved date fields)
        let storage = self.storage.lock().await;

        if let Some(task) = TaskRepository::get_by_id(&storage.conn, task_uuid).await? {
            let mut active_model: task::ActiveModel = task.into_active_model();
            Self::apply_backend_due_fields(&mut active_model, &backend_task);
            TaskRepository::update(&storage.conn, active_model).await?;
        }

        Ok(())
    }

    /// Update task priority
    pub async fn update_task_priority(&self, task_uuid: &Uuid, priority: i32) -> Result<()> {
        // Look up the task's remote_id for backend call
        let remote_id = self.get_task_remote_id(task_uuid).await?;

        // Update task via backend using the UpdateTaskArgs structure
        let task_args = crate::backend::UpdateTaskArgs {
            content: None,
            description: None,
            project_remote_id: None,
            section_remote_id: None,
            parent_remote_id: None,
            priority: Some(priority),
            due_date: None,
            due_datetime: None,
            due_string: None,
            duration: None,
            labels: None,
        };
        let _task = self
            .get_backend()
            .await?
            .update_task(&remote_id, task_args)
            .await
            .map_err(|e| anyhow::anyhow!("Backend error: {}", e))?;

        // Then update local storage
        let storage = self.storage.lock().await;

        if let Some(task) = TaskRepository::get_by_id(&storage.conn, task_uuid).await? {
            let mut active_model: task::ActiveModel = task.into_active_model();
            active_model.priority = ActiveValue::Set(priority);
            TaskRepository::update(&storage.conn, active_model).await?;
        }

        Ok(())
    }

    /// Marks a task as completed via the remote backend and removes it from local storage.
    ///
    /// This method completes the task remotely (which automatically handles subtasks)
    /// and removes it from local storage since completed tasks are not displayed in the UI.
    /// Subtasks are automatically deleted via database CASCADE constraints.
    ///
    /// # Arguments
    /// * `task_uuid` - The local UUID of the task to complete
    ///
    /// # Errors
    /// Returns an error if the backend call fails or local storage update fails
    pub async fn complete_task(&self, task_uuid: &Uuid) -> Result<()> {
        // Look up the task's remote_id for backend call
        let remote_id = self.get_task_remote_id(task_uuid).await?;

        // Complete the task via backend using remote_id (this handles subtasks automatically)
        self.get_backend()
            .await?
            .complete_task(&remote_id)
            .await
            .map_err(|e| anyhow::anyhow!("Backend error: {}", e))?;

        // Then mark as completed in local storage (soft completion)
        let storage = self.storage.lock().await;

        if let Some(task) = TaskRepository::get_by_id(&storage.conn, task_uuid).await? {
            let mut active_model: task::ActiveModel = task.into_active_model();
            active_model.is_completed = ActiveValue::Set(true);
            TaskRepository::update(&storage.conn, active_model).await?;
        }

        Ok(())
    }

    /// Permanently deletes a task via the remote backend and removes it from local storage.
    ///
    /// This method performs a hard delete of the task remotely, soft delete locally.
    /// The task will be permanently removed and cannot be recovered.
    ///
    /// # Arguments
    /// * `task_uuid` - The local UUID of the task to delete
    ///
    /// # Errors
    /// Returns an error if the backend call fails or local storage update fails
    pub async fn delete_task(&self, task_uuid: &Uuid) -> Result<()> {
        // Look up the task's remote_id for backend call
        let remote_id = self.get_task_remote_id(task_uuid).await?;

        // Delete the task via backend using remote_id
        self.get_backend()
            .await?
            .delete_task(&remote_id)
            .await
            .map_err(|e| anyhow::anyhow!("Backend error: {}", e))?;

        // Then mark as deleted in local storage (soft deletion)
        let storage = self.storage.lock().await;

        if let Some(task) = TaskRepository::get_by_id(&storage.conn, task_uuid).await? {
            let mut active_model: task::ActiveModel = task.into_active_model();
            active_model.is_deleted = ActiveValue::Set(true);
            TaskRepository::update(&storage.conn, active_model).await?;
        }

        Ok(())
    }

    /// Restore a soft-deleted or completed task via the remote backend and locally
    /// For completed tasks, reopens them. For deleted tasks, recreates them via backend.
    pub async fn restore_task(&self, task_id: &Uuid) -> Result<()> {
        // First, get the task from local storage to check its state
        let storage = self.storage.lock().await;
        let task = TaskRepository::get_by_id(&storage.conn, task_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Task not found in local storage: {}", task_id))?;

        if task.is_deleted {
            // For deleted tasks, we need to recreate them via backend
            // Look up remote IDs before dropping storage lock
            let remote_project_id = ProjectRepository::get_remote_id(&storage.conn, &task.project_uuid).await?;
            let remote_section_id = if let Some(section_uuid) = &task.section_uuid {
                SectionRepository::get_remote_id(&storage.conn, section_uuid).await?
            } else {
                None
            };
            let remote_parent_id = if let Some(parent_uuid) = &task.parent_uuid {
                Some(TaskRepository::get_remote_id(&storage.conn, parent_uuid).await?)
            } else {
                None
            };

            drop(storage); // Release the lock before API call

            // Create the task again via backend
            let task_args = crate::backend::CreateTaskArgs {
                content: task.content.clone(),
                description: task.description.clone().filter(|d| !d.is_empty()),
                project_remote_id: remote_project_id,
                section_remote_id: remote_section_id,
                parent_remote_id: remote_parent_id,
                priority: Some(task.priority),
                due_date: task.due_date.clone(),
                due_datetime: task.due_datetime.clone(),
                due_string: None,
                duration: task.duration.clone(),
                labels: Vec::new(), // Labels will be synced separately
            };

            let new_task = self
                .get_backend()
                .await?
                .create_task(task_args)
                .await
                .map_err(|e| anyhow::anyhow!("Backend error: {}", e))?;

            // Update local storage: remove the old soft-deleted task and add the new one
            let storage = self.storage.lock().await;

            // Hard delete the old soft-deleted task
            if let Some(old_task) = TaskRepository::get_by_id(&storage.conn, task_id).await? {
                TaskRepository::delete(&storage.conn, old_task).await?;
            }

            // Store the new task (reuse the single task upsert logic)
            let txn = storage.conn.begin().await?;

            let project_uuid =
                Self::lookup_project_uuid(&txn, &self.backend_uuid, &new_task.project_remote_id, "task restore")
                    .await?;

            let section_uuid =
                Self::lookup_section_uuid(&txn, &self.backend_uuid, new_task.section_remote_id.as_ref()).await?;

            let parent_uuid = if let Some(remote_parent_id) = &new_task.parent_remote_id {
                TaskRepository::get_by_remote_id(&txn, &self.backend_uuid, remote_parent_id)
                    .await?
                    .map(|t| t.uuid)
            } else {
                None
            };

            let local_task = task::ActiveModel {
                uuid: ActiveValue::Set(Uuid::new_v4()),
                backend_uuid: ActiveValue::Set(self.backend_uuid),
                remote_id: ActiveValue::Set(new_task.remote_id),
                content: ActiveValue::Set(new_task.content),
                description: ActiveValue::Set(new_task.description),
                project_uuid: ActiveValue::Set(project_uuid),
                section_uuid: ActiveValue::Set(section_uuid),
                parent_uuid: ActiveValue::Set(parent_uuid),
                priority: ActiveValue::Set(new_task.priority),
                order_index: ActiveValue::Set(new_task.order_index),
                due_date: ActiveValue::Set(new_task.due_date),
                due_datetime: ActiveValue::Set(new_task.due_datetime),
                is_recurring: ActiveValue::Set(new_task.is_recurring),
                deadline: ActiveValue::Set(new_task.deadline),
                duration: ActiveValue::Set(new_task.duration),
                is_completed: ActiveValue::Set(new_task.is_completed),
                is_deleted: ActiveValue::Set(false),
            };

            use sea_orm::sea_query::OnConflict;
            let mut insert = task::Entity::insert(local_task);
            insert = insert.on_conflict(
                OnConflict::columns([task::Column::BackendUuid, task::Column::RemoteId])
                    .update_columns([
                        task::Column::Content,
                        task::Column::Description,
                        task::Column::ProjectUuid,
                        task::Column::SectionUuid,
                        task::Column::ParentUuid,
                        task::Column::Priority,
                        task::Column::OrderIndex,
                        task::Column::DueDate,
                        task::Column::DueDatetime,
                        task::Column::IsRecurring,
                        task::Column::Deadline,
                        task::Column::Duration,
                        task::Column::IsCompleted,
                        task::Column::IsDeleted,
                    ])
                    .to_owned(),
            );
            insert.exec(&txn).await?;

            txn.commit().await?;
        } else {
            // For completed tasks, just reopen them
            let remote_id = task.remote_id.clone();
            drop(storage); // Release the lock before API call
            self.get_backend()
                .await?
                .reopen_task(&remote_id)
                .await
                .map_err(|e| anyhow::anyhow!("Backend error: {}", e))?;

            // Clear local completion flag
            let storage = self.storage.lock().await;

            if let Some(task) = TaskRepository::get_by_id(&storage.conn, task_id).await? {
                let mut active_model: task::ActiveModel = task.into_active_model();
                active_model.is_completed = ActiveValue::Set(false);
                TaskRepository::update(&storage.conn, active_model).await?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod update_task_full_tests {
    use super::*;
    use crate::backend::{
        Backend, BackendError, BackendLabel, BackendProject, BackendSection, BackendTask, CreateLabelArgs,
        CreateProjectArgs, CreateTaskArgs, UpdateLabelArgs, UpdateProjectArgs, UpdateTaskArgs,
    };
    use crate::backend_registry::BackendRegistry;
    use crate::entities::{backend, project, task};
    use crate::repositories::TaskRepository;
    use crate::storage::LocalStorage;
    use async_trait::async_trait;
    use sea_orm::{ActiveModelTrait, ActiveValue, ConnectionTrait, Database, DbBackend, Schema, Statement};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    struct StubBackend {
        updated_task: BackendTask,
    }

    #[async_trait]
    impl Backend for StubBackend {
        fn backend_type(&self) -> &str {
            "stub"
        }

        async fn fetch_projects(&self) -> Result<Vec<BackendProject>, BackendError> {
            panic!("not used in this test")
        }

        async fn fetch_tasks(&self) -> Result<Vec<BackendTask>, BackendError> {
            panic!("not used in this test")
        }

        async fn fetch_labels(&self) -> Result<Vec<BackendLabel>, BackendError> {
            panic!("not used in this test")
        }

        async fn fetch_sections(&self) -> Result<Vec<BackendSection>, BackendError> {
            panic!("not used in this test")
        }

        async fn create_project(&self, _args: CreateProjectArgs) -> Result<BackendProject, BackendError> {
            panic!("not used in this test")
        }

        async fn update_project(
            &self,
            _remote_id: &str,
            _args: UpdateProjectArgs,
        ) -> Result<BackendProject, BackendError> {
            panic!("not used in this test")
        }

        async fn delete_project(&self, _remote_id: &str) -> Result<(), BackendError> {
            panic!("not used in this test")
        }

        async fn create_task(&self, _args: CreateTaskArgs) -> Result<BackendTask, BackendError> {
            panic!("not used in this test")
        }

        async fn update_task(&self, _remote_id: &str, _args: UpdateTaskArgs) -> Result<BackendTask, BackendError> {
            Ok(self.updated_task.clone())
        }

        async fn delete_task(&self, _remote_id: &str) -> Result<(), BackendError> {
            panic!("not used in this test")
        }

        async fn complete_task(&self, _remote_id: &str) -> Result<(), BackendError> {
            panic!("not used in this test")
        }

        async fn reopen_task(&self, _remote_id: &str) -> Result<(), BackendError> {
            panic!("not used in this test")
        }

        async fn create_label(&self, _args: CreateLabelArgs) -> Result<BackendLabel, BackendError> {
            panic!("not used in this test")
        }

        async fn update_label(&self, _remote_id: &str, _args: UpdateLabelArgs) -> Result<BackendLabel, BackendError> {
            panic!("not used in this test")
        }

        async fn delete_label(&self, _remote_id: &str) -> Result<(), BackendError> {
            panic!("not used in this test")
        }
    }

    async fn setup_storage() -> anyhow::Result<Arc<Mutex<LocalStorage>>> {
        let conn = Database::connect("sqlite::memory:").await?;
        conn.execute(Statement::from_string(
            DbBackend::Sqlite,
            "PRAGMA foreign_keys = ON;".to_owned(),
        ))
        .await?;

        let schema = Schema::new(DbBackend::Sqlite);
        let table_statements = vec![
            schema.create_table_from_entity(backend::Entity),
            schema.create_table_from_entity(project::Entity),
            schema.create_table_from_entity(crate::entities::section::Entity),
            schema.create_table_from_entity(crate::entities::label::Entity),
            schema.create_table_from_entity(task::Entity),
            schema.create_table_from_entity(crate::entities::task_label::Entity),
        ];

        for statement in table_statements {
            conn.execute(DbBackend::Sqlite.build(&statement)).await?;
        }

        let indexes = vec![
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_projects_backend_remote ON projects(backend_uuid, remote_id)",
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_sections_backend_remote ON sections(backend_uuid, remote_id)",
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_labels_backend_remote ON labels(backend_uuid, remote_id)",
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_tasks_backend_remote ON tasks(backend_uuid, remote_id)",
        ];

        for index_sql in indexes {
            conn.execute(Statement::from_string(DbBackend::Sqlite, index_sql.to_owned()))
                .await?;
        }

        Ok(Arc::new(Mutex::new(LocalStorage { conn })))
    }

    async fn insert_backend_project_task(
        conn: &sea_orm::DatabaseConnection,
        backend_uuid: Uuid,
        project_uuid: Uuid,
        project_remote_id: &str,
        task_uuid: Uuid,
        task_remote_id: &str,
    ) -> anyhow::Result<()> {
        backend::ActiveModel {
            uuid: ActiveValue::Set(backend_uuid),
            backend_type: ActiveValue::Set("stub".to_string()),
            name: ActiveValue::Set("Stub Backend".to_string()),
            is_enabled: ActiveValue::Set(true),
            credentials: ActiveValue::Set("{}".to_string()),
            settings: ActiveValue::Set("{}".to_string()),
        }
        .insert(conn)
        .await?;

        project::ActiveModel {
            uuid: ActiveValue::Set(project_uuid),
            backend_uuid: ActiveValue::Set(backend_uuid),
            remote_id: ActiveValue::Set(project_remote_id.to_string()),
            name: ActiveValue::Set("Project".to_string()),
            is_favorite: ActiveValue::Set(false),
            is_inbox_project: ActiveValue::Set(false),
            order_index: ActiveValue::Set(0),
            parent_uuid: ActiveValue::Set(None),
        }
        .insert(conn)
        .await?;

        task::ActiveModel {
            uuid: ActiveValue::Set(task_uuid),
            backend_uuid: ActiveValue::Set(backend_uuid),
            remote_id: ActiveValue::Set(task_remote_id.to_string()),
            content: ActiveValue::Set("Local original content".to_string()),
            description: ActiveValue::Set(None),
            project_uuid: ActiveValue::Set(project_uuid),
            section_uuid: ActiveValue::Set(None),
            parent_uuid: ActiveValue::Set(None),
            priority: ActiveValue::Set(1),
            order_index: ActiveValue::Set(0),
            due_date: ActiveValue::Set(None),
            due_datetime: ActiveValue::Set(None),
            is_recurring: ActiveValue::Set(false),
            deadline: ActiveValue::Set(None),
            duration: ActiveValue::Set(None),
            is_completed: ActiveValue::Set(false),
            is_deleted: ActiveValue::Set(false),
        }
        .insert(conn)
        .await?;

        Ok(())
    }

    fn backend_task_with_project(project_remote_id: &str) -> BackendTask {
        BackendTask {
            remote_id: "remote-task-1".to_string(),
            content: "Remote updated content".to_string(),
            description: Some("Remote updated description".to_string()),
            project_remote_id: project_remote_id.to_string(),
            section_remote_id: None,
            parent_remote_id: None,
            priority: 1,
            order_index: 0,
            due_date: Some("2026-03-01".to_string()),
            due_datetime: None,
            is_recurring: false,
            deadline: None,
            duration: None,
            is_completed: false,
            labels: Vec::new(),
        }
    }

    #[tokio::test]
    async fn update_task_full_errors_when_backend_project_mapping_is_missing() {
        let storage = setup_storage().await.expect("storage setup should succeed");
        let backend_uuid = Uuid::new_v4();
        let local_project_uuid = Uuid::new_v4();
        let task_uuid = Uuid::new_v4();

        {
            let guard = storage.lock().await;
            insert_backend_project_task(
                &guard.conn,
                backend_uuid,
                local_project_uuid,
                "known-project",
                task_uuid,
                "remote-task-1",
            )
            .await
            .expect("seed data should succeed");
        }

        let backend_registry = Arc::new(BackendRegistry::new(storage.clone()));
        backend_registry
            .register_backend_for_test(
                backend_uuid,
                Box::new(StubBackend {
                    updated_task: backend_task_with_project("missing-project"),
                }),
            )
            .await;

        let sync_service = SyncService::new(backend_registry, backend_uuid, false)
            .await
            .expect("sync service should initialize");

        let error = sync_service
            .update_task_full(
                &task_uuid,
                "User updated content",
                Some("User updated description"),
                None,
                ProjectUpdateIntent::Set(local_project_uuid),
            )
            .await
            .expect_err("missing project mapping should return an error");

        assert!(error
            .to_string()
            .contains("Project with remote_id missing-project not found locally during task update"));

        let guard = storage.lock().await;
        let task = TaskRepository::get_by_id(&guard.conn, &task_uuid)
            .await
            .expect("task query should succeed")
            .expect("task should exist");
        assert_eq!(task.content, "Local original content");
        assert_eq!(task.description, None);
        assert_eq!(task.project_uuid, local_project_uuid);
    }

    #[tokio::test]
    async fn update_task_full_updates_task_when_backend_project_mapping_exists() {
        let storage = setup_storage().await.expect("storage setup should succeed");
        let backend_uuid = Uuid::new_v4();
        let local_project_uuid = Uuid::new_v4();
        let mapped_project_uuid = Uuid::new_v4();
        let task_uuid = Uuid::new_v4();

        {
            let guard = storage.lock().await;
            insert_backend_project_task(
                &guard.conn,
                backend_uuid,
                local_project_uuid,
                "known-project",
                task_uuid,
                "remote-task-1",
            )
            .await
            .expect("seed data should succeed");

            project::ActiveModel {
                uuid: ActiveValue::Set(mapped_project_uuid),
                backend_uuid: ActiveValue::Set(backend_uuid),
                remote_id: ActiveValue::Set("mapped-project".to_string()),
                name: ActiveValue::Set("Mapped Project".to_string()),
                is_favorite: ActiveValue::Set(false),
                is_inbox_project: ActiveValue::Set(false),
                order_index: ActiveValue::Set(1),
                parent_uuid: ActiveValue::Set(None),
            }
            .insert(&guard.conn)
            .await
            .expect("mapped project insert should succeed");
        }

        let backend_registry = Arc::new(BackendRegistry::new(storage.clone()));
        backend_registry
            .register_backend_for_test(
                backend_uuid,
                Box::new(StubBackend {
                    updated_task: backend_task_with_project("mapped-project"),
                }),
            )
            .await;

        let sync_service = SyncService::new(backend_registry, backend_uuid, false)
            .await
            .expect("sync service should initialize");

        sync_service
            .update_task_full(
                &task_uuid,
                "User updated content",
                Some("User updated description"),
                None,
                ProjectUpdateIntent::Set(local_project_uuid),
            )
            .await
            .expect("update should succeed when mapping exists");

        let guard = storage.lock().await;
        let task = TaskRepository::get_by_id(&guard.conn, &task_uuid)
            .await
            .expect("task query should succeed")
            .expect("task should exist");
        assert_eq!(task.content, "Remote updated content");
        assert_eq!(task.description, Some("Remote updated description".to_string()));
        assert_eq!(task.project_uuid, mapped_project_uuid);
        assert_eq!(task.due_date, Some("2026-03-01".to_string()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::{
        Backend, BackendError, BackendLabel, BackendProject, BackendSection, BackendTask, CreateLabelArgs,
        CreateProjectArgs, CreateTaskArgs, UpdateLabelArgs, UpdateProjectArgs, UpdateTaskArgs,
    };
    use crate::backend_registry::BackendRegistry;
    use crate::entities::{backend, label, project, section, task, task_label};
    use crate::storage::LocalStorage;
    use async_trait::async_trait;
    use sea_orm::{
        ActiveModelTrait, ColumnTrait, ConnectionTrait, Database, DatabaseConnection, DbBackend, EntityTrait,
        QueryFilter, Schema, Statement,
    };
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[derive(Default)]
    struct MockCapture {
        due_string: Option<Option<String>>,
        project_remote_id: Option<Option<String>>,
        create_args: Option<CreateTaskArgs>,
    }

    struct MockBackend {
        update_response: BackendTask,
        create_response: Option<BackendTask>,
        update_error: Option<String>,
        capture: Arc<Mutex<MockCapture>>,
    }

    impl MockBackend {
        fn new(update_response: BackendTask, capture: Arc<Mutex<MockCapture>>) -> Self {
            Self {
                update_response,
                create_response: None,
                update_error: None,
                capture,
            }
        }

        fn with_create_response(mut self, response: BackendTask) -> Self {
            self.create_response = Some(response);
            self
        }
    }

    #[async_trait]
    impl Backend for MockBackend {
        fn backend_type(&self) -> &str {
            "mock"
        }

        async fn fetch_projects(&self) -> Result<Vec<BackendProject>, BackendError> {
            Ok(vec![])
        }

        async fn fetch_tasks(&self) -> Result<Vec<BackendTask>, BackendError> {
            Ok(vec![])
        }

        async fn fetch_labels(&self) -> Result<Vec<BackendLabel>, BackendError> {
            Ok(vec![])
        }

        async fn fetch_sections(&self) -> Result<Vec<BackendSection>, BackendError> {
            Ok(vec![])
        }

        async fn create_project(&self, _args: CreateProjectArgs) -> Result<BackendProject, BackendError> {
            Err(BackendError::Other("not implemented".to_string()))
        }

        async fn update_project(
            &self,
            _remote_id: &str,
            _args: UpdateProjectArgs,
        ) -> Result<BackendProject, BackendError> {
            Err(BackendError::Other("not implemented".to_string()))
        }

        async fn delete_project(&self, _remote_id: &str) -> Result<(), BackendError> {
            Err(BackendError::Other("not implemented".to_string()))
        }

        async fn create_task(&self, args: CreateTaskArgs) -> Result<BackendTask, BackendError> {
            let response = self
                .create_response
                .clone()
                .ok_or_else(|| BackendError::Other("create_task not configured".to_string()))?;
            let mut capture = self.capture.lock().await;
            capture.create_args = Some(args);
            Ok(response)
        }

        async fn update_task(&self, remote_id: &str, args: UpdateTaskArgs) -> Result<BackendTask, BackendError> {
            let mut capture = self.capture.lock().await;
            let _ = remote_id;
            capture.due_string = Some(args.due_string.clone());
            capture.project_remote_id = Some(args.project_remote_id.clone());

            if let Some(message) = &self.update_error {
                return Err(BackendError::Other(message.clone()));
            }

            Ok(self.update_response.clone())
        }

        async fn delete_task(&self, _remote_id: &str) -> Result<(), BackendError> {
            Err(BackendError::Other("not implemented".to_string()))
        }

        async fn complete_task(&self, _remote_id: &str) -> Result<(), BackendError> {
            Err(BackendError::Other("not implemented".to_string()))
        }

        async fn reopen_task(&self, _remote_id: &str) -> Result<(), BackendError> {
            Err(BackendError::Other("not implemented".to_string()))
        }

        async fn create_label(&self, _args: CreateLabelArgs) -> Result<BackendLabel, BackendError> {
            Err(BackendError::Other("not implemented".to_string()))
        }

        async fn update_label(&self, _remote_id: &str, _args: UpdateLabelArgs) -> Result<BackendLabel, BackendError> {
            Err(BackendError::Other("not implemented".to_string()))
        }

        async fn delete_label(&self, _remote_id: &str) -> Result<(), BackendError> {
            Err(BackendError::Other("not implemented".to_string()))
        }
    }

    async fn setup_conn() -> DatabaseConnection {
        let conn = Database::connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite should connect");

        conn.execute(Statement::from_string(
            DbBackend::Sqlite,
            "PRAGMA foreign_keys = ON;".to_owned(),
        ))
        .await
        .expect("should enable sqlite foreign keys");

        let backend_db = conn.get_database_backend();
        let schema = Schema::new(backend_db);
        let table_statements = [
            schema.create_table_from_entity(backend::Entity),
            schema.create_table_from_entity(project::Entity),
            schema.create_table_from_entity(section::Entity),
            schema.create_table_from_entity(label::Entity),
            schema.create_table_from_entity(task::Entity),
            schema.create_table_from_entity(task_label::Entity),
        ];

        for statement in table_statements {
            conn.execute(backend_db.build(&statement))
                .await
                .expect("should create test table");
        }

        let indexes = [
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_projects_backend_remote ON projects(backend_uuid, remote_id)",
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_sections_backend_remote ON sections(backend_uuid, remote_id)",
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_labels_backend_remote ON labels(backend_uuid, remote_id)",
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_tasks_backend_remote ON tasks(backend_uuid, remote_id)",
        ];
        for index_sql in indexes {
            conn.execute(Statement::from_string(DbBackend::Sqlite, index_sql.to_string()))
                .await
                .expect("should create test index");
        }

        conn
    }

    async fn seed_backend(conn: &DatabaseConnection, backend_uuid: Uuid) {
        backend::ActiveModel {
            uuid: ActiveValue::Set(backend_uuid),
            backend_type: ActiveValue::Set("mock".to_string()),
            name: ActiveValue::Set("Mock backend".to_string()),
            is_enabled: ActiveValue::Set(true),
            credentials: ActiveValue::Set("{}".to_string()),
            settings: ActiveValue::Set("{}".to_string()),
        }
        .insert(conn)
        .await
        .expect("should seed backend");
    }

    async fn seed_project(
        conn: &DatabaseConnection,
        backend_uuid: Uuid,
        remote_id: &str,
        is_inbox_project: bool,
    ) -> Uuid {
        let project_uuid = Uuid::new_v4();
        project::ActiveModel {
            uuid: ActiveValue::Set(project_uuid),
            backend_uuid: ActiveValue::Set(backend_uuid),
            remote_id: ActiveValue::Set(remote_id.to_string()),
            name: ActiveValue::Set(remote_id.to_string()),
            is_favorite: ActiveValue::Set(false),
            is_inbox_project: ActiveValue::Set(is_inbox_project),
            order_index: ActiveValue::Set(1),
            parent_uuid: ActiveValue::Set(None),
        }
        .insert(conn)
        .await
        .expect("should seed project");

        project_uuid
    }

    async fn seed_task(
        conn: &DatabaseConnection,
        backend_uuid: Uuid,
        project_uuid: Uuid,
        due_date: Option<&str>,
    ) -> Uuid {
        let task_uuid = Uuid::new_v4();
        task::ActiveModel {
            uuid: ActiveValue::Set(task_uuid),
            backend_uuid: ActiveValue::Set(backend_uuid),
            remote_id: ActiveValue::Set("task-1".to_string()),
            content: ActiveValue::Set("Initial content".to_string()),
            description: ActiveValue::Set(Some("Initial description".to_string())),
            project_uuid: ActiveValue::Set(project_uuid),
            section_uuid: ActiveValue::Set(None),
            parent_uuid: ActiveValue::Set(None),
            priority: ActiveValue::Set(1),
            order_index: ActiveValue::Set(1),
            due_date: ActiveValue::Set(due_date.map(std::string::ToString::to_string)),
            due_datetime: ActiveValue::Set(None),
            is_recurring: ActiveValue::Set(false),
            deadline: ActiveValue::Set(Some("2026-03-05".to_string())),
            duration: ActiveValue::Set(None),
            is_completed: ActiveValue::Set(false),
            is_deleted: ActiveValue::Set(false),
        }
        .insert(conn)
        .await
        .expect("should seed task");

        task_uuid
    }

    fn backend_task(project_remote_id: &str, due_date: Option<&str>, deadline: Option<&str>) -> BackendTask {
        BackendTask {
            remote_id: "task-1".to_string(),
            content: "Updated content".to_string(),
            description: Some("Updated description".to_string()),
            project_remote_id: project_remote_id.to_string(),
            section_remote_id: None,
            parent_remote_id: None,
            priority: 1,
            order_index: 1,
            due_date: due_date.map(std::string::ToString::to_string),
            due_datetime: None,
            is_recurring: false,
            deadline: deadline.map(std::string::ToString::to_string),
            duration: None,
            is_completed: false,
            labels: vec![],
        }
    }

    async fn setup_sync_service(
        response: BackendTask,
    ) -> (SyncService, DatabaseConnection, Arc<Mutex<MockCapture>>, Uuid) {
        let conn = setup_conn().await;
        let backend_uuid = Uuid::new_v4();
        seed_backend(&conn, backend_uuid).await;
        let capture = Arc::new(Mutex::new(MockCapture::default()));

        let storage = Arc::new(Mutex::new(LocalStorage { conn: conn.clone() }));
        let registry = Arc::new(BackendRegistry::new(storage));
        registry
            .register_backend_for_test(backend_uuid, Box::new(MockBackend::new(response, capture.clone())))
            .await;

        let sync_service = SyncService::new(registry, backend_uuid, false)
            .await
            .expect("sync service should be created");

        (sync_service, conn, capture, backend_uuid)
    }

    async fn fetch_task(conn: &DatabaseConnection, task_uuid: Uuid) -> task::Model {
        task::Entity::find()
            .filter(task::Column::Uuid.eq(task_uuid))
            .one(conn)
            .await
            .expect("query should succeed")
            .expect("task should exist")
    }

    fn backend_task_with_due(deadline: Option<&str>) -> crate::backend::BackendTask {
        crate::backend::BackendTask {
            remote_id: "task-1".to_string(),
            content: "Task".to_string(),
            description: None,
            project_remote_id: "project-1".to_string(),
            section_remote_id: None,
            parent_remote_id: None,
            priority: 1,
            order_index: 1,
            due_date: Some("2026-03-01".to_string()),
            due_datetime: Some("2026-03-01T09:00:00Z".to_string()),
            is_recurring: true,
            deadline: deadline.map(std::string::ToString::to_string),
            duration: None,
            is_completed: false,
            labels: vec![],
        }
    }

    #[test]
    fn apply_backend_due_fields_sets_deadline() {
        let backend_task = backend_task_with_due(Some("2026-03-05"));
        let mut active_model: task::ActiveModel = Default::default();

        SyncService::apply_backend_due_fields(&mut active_model, &backend_task);

        assert_eq!(active_model.due_date, ActiveValue::Set(Some("2026-03-01".to_string())));
        assert_eq!(
            active_model.due_datetime,
            ActiveValue::Set(Some("2026-03-01T09:00:00Z".to_string()))
        );
        assert_eq!(active_model.is_recurring, ActiveValue::Set(true));
        assert_eq!(active_model.deadline, ActiveValue::Set(Some("2026-03-05".to_string())));
    }

    #[test]
    fn apply_backend_due_fields_clears_deadline() {
        let backend_task = backend_task_with_due(None);
        let mut active_model: task::ActiveModel = Default::default();

        active_model.deadline = ActiveValue::Set(Some("2026-03-05".to_string()));
        SyncService::apply_backend_due_fields(&mut active_model, &backend_task);

        assert_eq!(active_model.deadline, ActiveValue::Set(None));
    }

    #[tokio::test]
    async fn update_task_full_no_change_due_sends_none_and_keeps_due() {
        let (sync_service, conn, capture, backend_uuid) =
            setup_sync_service(backend_task("project-a", Some("2026-03-10"), Some("2026-03-12"))).await;
        let project_uuid = seed_project(&conn, backend_uuid, "project-a", false).await;
        let task_uuid = seed_task(&conn, backend_uuid, project_uuid, Some("2026-03-10")).await;

        sync_service
            .update_task_full(
                &task_uuid,
                "Updated content",
                Some("Updated description"),
                None,
                ProjectUpdateIntent::Unchanged,
            )
            .await
            .expect("update_task_full should succeed");

        let captured = capture.lock().await;
        assert_eq!(captured.due_string, Some(None));
        assert_eq!(captured.project_remote_id, Some(None));
        drop(captured);

        let updated = fetch_task(&conn, task_uuid).await;
        assert_eq!(updated.due_date.as_deref(), Some("2026-03-10"));
        assert_eq!(updated.deadline.as_deref(), Some("2026-03-12"));
    }

    #[tokio::test]
    async fn update_task_full_clear_due_sends_no_date_and_clears_due() {
        let (sync_service, conn, capture, backend_uuid) =
            setup_sync_service(backend_task("project-a", None, None)).await;
        let project_uuid = seed_project(&conn, backend_uuid, "project-a", false).await;
        let task_uuid = seed_task(&conn, backend_uuid, project_uuid, Some("2026-03-10")).await;

        sync_service
            .update_task_full(
                &task_uuid,
                "Updated content",
                Some("Updated description"),
                Some("no date"),
                ProjectUpdateIntent::Unchanged,
            )
            .await
            .expect("update_task_full should succeed");

        let captured = capture.lock().await;
        assert_eq!(captured.due_string, Some(Some("no date".to_string())));
        assert_eq!(captured.project_remote_id, Some(None));
        drop(captured);

        let updated = fetch_task(&conn, task_uuid).await;
        assert!(updated.due_date.is_none());
        assert!(updated.due_datetime.is_none());
        assert!(updated.deadline.is_none());
    }

    #[tokio::test]
    async fn update_task_full_project_mapping_success_updates_project_and_deadline() {
        let (sync_service, conn, capture, backend_uuid) =
            setup_sync_service(backend_task("project-b", Some("2026-03-10"), Some("2026-03-14"))).await;
        let project_a_uuid = seed_project(&conn, backend_uuid, "project-a", false).await;
        let project_b_uuid = seed_project(&conn, backend_uuid, "project-b", false).await;
        let task_uuid = seed_task(&conn, backend_uuid, project_a_uuid, Some("2026-03-08")).await;

        sync_service
            .update_task_full(
                &task_uuid,
                "Updated content",
                Some("Updated description"),
                None,
                ProjectUpdateIntent::Set(project_b_uuid),
            )
            .await
            .expect("update_task_full should succeed");

        let captured = capture.lock().await;
        assert_eq!(captured.project_remote_id, Some(Some("project-b".to_string())));
        drop(captured);

        let updated = fetch_task(&conn, task_uuid).await;
        assert_eq!(updated.project_uuid, project_b_uuid);
        assert_eq!(updated.deadline.as_deref(), Some("2026-03-14"));
    }

    #[tokio::test]
    async fn update_task_full_project_mapping_failure_returns_error() {
        let (sync_service, conn, _capture, backend_uuid) =
            setup_sync_service(backend_task("project-missing", Some("2026-03-10"), Some("2026-03-14"))).await;
        let project_a_uuid = seed_project(&conn, backend_uuid, "project-a", false).await;
        let task_uuid = seed_task(&conn, backend_uuid, project_a_uuid, Some("2026-03-08")).await;

        let result = sync_service
            .update_task_full(
                &task_uuid,
                "Updated content",
                Some("Updated description"),
                None,
                ProjectUpdateIntent::Unchanged,
            )
            .await;

        assert!(result.is_err());
        let error_text = format!("{}", result.err().expect("error should exist"));
        assert!(error_text.contains("not found locally during task update"));

        let unchanged = fetch_task(&conn, task_uuid).await;
        assert_eq!(unchanged.project_uuid, project_a_uuid);
        assert_eq!(unchanged.content, "Initial content");
        assert_eq!(unchanged.deadline.as_deref(), Some("2026-03-05"));
    }

    // ---------------------------------------------------------------------------
    // Suite 6: create_task
    // ---------------------------------------------------------------------------

    async fn setup_sync_service_with_create(
        update_response: BackendTask,
        create_response: BackendTask,
    ) -> (SyncService, DatabaseConnection, Arc<Mutex<MockCapture>>, Uuid) {
        let conn = setup_conn().await;
        let backend_uuid = Uuid::new_v4();
        seed_backend(&conn, backend_uuid).await;
        let capture = Arc::new(Mutex::new(MockCapture::default()));

        let mock = MockBackend::new(update_response, capture.clone()).with_create_response(create_response);

        let storage = Arc::new(Mutex::new(LocalStorage { conn: conn.clone() }));
        let registry = Arc::new(BackendRegistry::new(storage));
        registry
            .register_backend_for_test(backend_uuid, Box::new(mock))
            .await;

        let sync_service = SyncService::new(registry, backend_uuid, false)
            .await
            .expect("sync service should be created");

        (sync_service, conn, capture, backend_uuid)
    }

    #[tokio::test]
    async fn create_task_passes_due_string_and_description_to_backend() {
        let create_response = BackendTask {
            remote_id: "new-task-1".to_string(),
            content: "Buy milk".to_string(),
            description: Some("Organic".to_string()),
            project_remote_id: "project-a".to_string(),
            section_remote_id: None,
            parent_remote_id: None,
            priority: 1,
            order_index: 1,
            due_date: Some("2026-03-01".to_string()),
            due_datetime: None,
            is_recurring: false,
            deadline: None,
            duration: None,
            is_completed: false,
            labels: vec![],
        };
        let (sync_service, conn, capture, backend_uuid) = setup_sync_service_with_create(
            backend_task("project-a", None, None),
            create_response,
        )
        .await;
        let project_uuid = seed_project(&conn, backend_uuid, "project-a", false).await;

        sync_service
            .create_task("Buy milk", Some("Organic"), Some("tomorrow"), Some(project_uuid))
            .await
            .expect("create_task should succeed");

        let captured = capture.lock().await;
        let args = captured.create_args.as_ref().expect("create_args should be captured");
        assert_eq!(args.due_string.as_deref(), Some("tomorrow"));
        assert_eq!(args.description.as_deref(), Some("Organic"));
        assert_eq!(args.content, "Buy milk");
        assert_eq!(args.project_remote_id, "project-a");
    }

    #[tokio::test]
    async fn create_task_with_none_optionals() {
        let create_response = BackendTask {
            remote_id: "new-task-2".to_string(),
            content: "Simple task".to_string(),
            description: None,
            project_remote_id: "inbox-project".to_string(),
            section_remote_id: None,
            parent_remote_id: None,
            priority: 1,
            order_index: 1,
            due_date: None,
            due_datetime: None,
            is_recurring: false,
            deadline: None,
            duration: None,
            is_completed: false,
            labels: vec![],
        };
        let (sync_service, conn, capture, backend_uuid) = setup_sync_service_with_create(
            backend_task("inbox-project", None, None),
            create_response,
        )
        .await;
        // Seed only an inbox project (no project UUID passed)
        seed_project(&conn, backend_uuid, "inbox-project", true).await;

        sync_service
            .create_task("Simple task", None, None, None)
            .await
            .expect("create_task should succeed");

        let captured = capture.lock().await;
        let args = captured.create_args.as_ref().expect("create_args should be captured");
        assert_eq!(args.due_string, None);
        assert_eq!(args.description, None);
        // No project UUID was provided, so project_remote_id defaults to empty string
        assert!(args.project_remote_id.is_empty());
    }

    // ---------------------------------------------------------------------------
    // Suite 7: update_task_full MoveToInbox
    // ---------------------------------------------------------------------------

    #[tokio::test]
    async fn update_task_full_move_to_inbox_resolves_inbox_project() {
        let (sync_service, conn, capture, backend_uuid) =
            setup_sync_service(backend_task("inbox-remote", Some("2026-03-10"), None)).await;
        // Seed inbox project and a regular project
        let inbox_uuid = seed_project(&conn, backend_uuid, "inbox-remote", true).await;
        let project_a_uuid = seed_project(&conn, backend_uuid, "project-a", false).await;
        let task_uuid = seed_task(&conn, backend_uuid, project_a_uuid, Some("2026-03-08")).await;

        sync_service
            .update_task_full(
                &task_uuid,
                "Updated content",
                Some("Updated description"),
                None,
                ProjectUpdateIntent::MoveToInbox,
            )
            .await
            .expect("update_task_full should succeed");

        let captured = capture.lock().await;
        assert_eq!(
            captured.project_remote_id,
            Some(Some("inbox-remote".to_string())),
            "MoveToInbox should resolve to inbox project's remote_id"
        );
        drop(captured);

        let updated = fetch_task(&conn, task_uuid).await;
        assert_eq!(updated.project_uuid, inbox_uuid);
    }
}
