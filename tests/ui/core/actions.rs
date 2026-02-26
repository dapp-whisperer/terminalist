use terminalist::sync::tasks::ProjectUpdateIntent;
use terminalist::ui::core::actions::Action;
use uuid::Uuid;

#[test]
fn test_action_enum_exists() {
    // Test that Action enum is accessible and has a valid size
    let action_size = std::mem::size_of::<Action>();
    // Action enum should have a non-zero size
    assert!(action_size > 0, "Action enum should have a non-zero size");
}

#[test]
fn edit_task_supports_all_project_update_intents() {
    let task_uuid = Uuid::new_v4();

    let unchanged = Action::EditTask {
        task_uuid,
        content: "task".to_string(),
        description: None,
        due_string: None,
        project_update: ProjectUpdateIntent::Unchanged,
    };

    let move_to_inbox = Action::EditTask {
        task_uuid,
        content: "task".to_string(),
        description: None,
        due_string: None,
        project_update: ProjectUpdateIntent::MoveToInbox,
    };

    let set_project_uuid = Uuid::new_v4();
    let set_project = Action::EditTask {
        task_uuid,
        content: "task".to_string(),
        description: None,
        due_string: None,
        project_update: ProjectUpdateIntent::Set(set_project_uuid),
    };

    assert!(matches!(
        unchanged,
        Action::EditTask {
            project_update: ProjectUpdateIntent::Unchanged,
            ..
        }
    ));
    assert!(matches!(
        move_to_inbox,
        Action::EditTask {
            project_update: ProjectUpdateIntent::MoveToInbox,
            ..
        }
    ));
    assert!(matches!(
        set_project,
        Action::EditTask {
            project_update: ProjectUpdateIntent::Set(uuid),
            ..
        } if uuid == set_project_uuid
    ));
}
