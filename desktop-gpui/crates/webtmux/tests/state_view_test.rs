use webtmux::app_state::AppState;
use webtmux::views::session_states::{determine_workspace_state, WorkspaceState};
use webtmux_backend_client::{SessionTreeNode, TmuxSession, TmuxTree};
use webtmux_settings::DesktopSettings;

#[test]
fn test_workspace_state_routing() {
    let settings = DesktopSettings::default();
    let mut app = AppState::new(settings, None);

    // 1. When tree_error is present -> routes to Error
    app.tree_error = Some("Tmux is not installed".to_string());
    assert_eq!(app.workspace_state(), WorkspaceState::Error);
    assert_eq!(
        determine_workspace_state(app.tree_error.as_deref(), false, false),
        WorkspaceState::Error
    );

    // 2. When tree_error is None and tree.sessions is empty -> routes to Empty
    app.tree_error = None;
    app.tree = TmuxTree::default();
    app.active_session = None;
    assert_eq!(app.workspace_state(), WorkspaceState::Empty);
    assert_eq!(
        determine_workspace_state(None, false, false),
        WorkspaceState::Empty
    );

    // 3. When tree_error is None, sessions exist, but active_session is None -> routes to SelectSession
    let session = SessionTreeNode {
        session: TmuxSession {
            name: "dev".to_string(),
            windows: 2,
            attached: 0,
            created_at: 1717200000,
            width: 80,
            height: 24,
        },
        windows: vec![],
    };
    app.tree.sessions = vec![session];
    app.active_session = None;
    assert_eq!(app.workspace_state(), WorkspaceState::SelectSession);
    assert_eq!(
        determine_workspace_state(None, true, false),
        WorkspaceState::SelectSession
    );

    // 4. When tree_error is None and active_session is set -> routes to ActiveSession
    app.active_session = Some("dev".to_string());
    assert_eq!(app.workspace_state(), WorkspaceState::ActiveSession);
    assert_eq!(
        determine_workspace_state(None, true, true),
        WorkspaceState::ActiveSession
    );
}
