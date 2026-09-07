//! WS envelope shapes for all 15 pane/window commands (PANE-02/03/05/06/08).
//!
//! Locks the `WsIncoming` JSON contract against `be/internal/realtime/protocol.go`:
//! `pane.split` direction strings, `paneId`-carried `@N` window targets, six
//! layout strings incl `next-layout`, and asserts no `pane.join` exists (D8).

use webtmux_backend_client::{
    WsIncoming, MSG_PANE_BREAK, MSG_PANE_KILL, MSG_PANE_RENAME, MSG_PANE_RESIZE,
    MSG_PANE_SELECT, MSG_PANE_SPLIT, MSG_PANE_SWAP, MSG_PANE_ZOOM, MSG_WINDOW_BREAK_ACTIVE,
    MSG_WINDOW_CREATE, MSG_WINDOW_KILL, MSG_WINDOW_LAYOUT, MSG_WINDOW_MOVE,
    MSG_WINDOW_RENAME, MSG_WINDOW_SELECT,
};

fn json(msg: &WsIncoming) -> serde_json::Value {
    serde_json::to_value(msg).expect("serializable")
}

#[test]
fn test_ws_pane_window_envelopes() {
    // Pane selects carry the %N id.
    let m = WsIncoming {
        msg_type: MSG_PANE_SELECT.to_string(),
        pane_id: Some("%3".to_string()),
        ..Default::default()
    };
    let v = json(&m);
    assert_eq!(v["type"], "pane.select");
    assert_eq!(v["paneId"], "%3");

    // Split direction strings: right -> horizontal, down -> vertical.
    for dir in ["horizontal", "vertical"] {
        let m = WsIncoming {
            msg_type: MSG_PANE_SPLIT.to_string(),
            pane_id: Some("%3".to_string()),
            direction: Some(dir.to_string()),
            ..Default::default()
        };
        let v = json(&m);
        assert_eq!(v["type"], "pane.split");
        assert_eq!(v["direction"], dir);
    }

    // Resize carries single-letter direction + amount.
    for dir in ["L", "R", "U", "D"] {
        let m = WsIncoming {
            msg_type: MSG_PANE_RESIZE.to_string(),
            pane_id: Some("%1".to_string()),
            direction: Some(dir.to_string()),
            amount: Some(3),
            ..Default::default()
        };
        let v = json(&m);
        assert_eq!(v["type"], "pane.resize");
        assert_eq!(v["direction"], dir);
        assert_eq!(v["amount"], 3);
    }

    // Kill / zoom / break carry only the pane id.
    for ty in [MSG_PANE_KILL, MSG_PANE_ZOOM, MSG_PANE_BREAK] {
        let m = WsIncoming {
            msg_type: ty.to_string(),
            pane_id: Some("%2".to_string()),
            ..Default::default()
        };
        let v = json(&m);
        assert_eq!(v["paneId"], "%2");
    }
    assert_eq!(MSG_PANE_KILL, "pane.kill");
    assert_eq!(MSG_PANE_ZOOM, "pane.zoom");
    assert_eq!(MSG_PANE_BREAK, "pane.break");

    // Rename carries title (argv, never shell — T-05-01).
    let m = WsIncoming {
        msg_type: MSG_PANE_RENAME.to_string(),
        pane_id: Some("%1".to_string()),
        title: Some("editor".to_string()),
        ..Default::default()
    };
    let v = json(&m);
    assert_eq!(v["type"], "pane.rename");
    assert_eq!(v["title"], "editor");

    // Swap carries otherPaneId.
    let m = WsIncoming {
        msg_type: MSG_PANE_SWAP.to_string(),
        pane_id: Some("%1".to_string()),
        other_pane_id: Some("%2".to_string()),
        ..Default::default()
    };
    let v = json(&m);
    assert_eq!(v["type"], "pane.swap");
    assert_eq!(v["otherPaneId"], "%2");

    // Window targets ride paneId with the @N id (no windowId field exists).
    for ty in [
        MSG_WINDOW_SELECT,
        MSG_WINDOW_RENAME,
        MSG_WINDOW_KILL,
        MSG_WINDOW_LAYOUT,
        MSG_WINDOW_MOVE,
        MSG_WINDOW_BREAK_ACTIVE,
    ] {
        let m = WsIncoming {
            msg_type: ty.to_string(),
            pane_id: Some("@2".to_string()),
            ..Default::default()
        };
        let v = json(&m);
        assert_eq!(v["paneId"], "@2");
        assert!(v.get("windowId").is_none(), "no windowId field exists");
    }
    assert_eq!(MSG_WINDOW_SELECT, "window.select");

    // Window create carries name/cwd/command, no paneId.
    let m = WsIncoming {
        msg_type: MSG_WINDOW_CREATE.to_string(),
        name: Some("dev".to_string()),
        ..Default::default()
    };
    let v = json(&m);
    assert_eq!(v["type"], "window.create");
    assert_eq!(v["name"], "dev");

    // Window rename carries name.
    let m = WsIncoming {
        msg_type: MSG_WINDOW_RENAME.to_string(),
        pane_id: Some("@1".to_string()),
        name: Some("main".to_string()),
        ..Default::default()
    };
    let v = json(&m);
    assert_eq!(v["type"], "window.rename");
    assert_eq!(v["name"], "main");

    // All six layout strings pass through verbatim incl next-layout (D7).
    for layout in [
        "even-horizontal",
        "even-vertical",
        "main-horizontal",
        "main-vertical",
        "tiled",
        "next-layout",
    ] {
        let m = WsIncoming {
            msg_type: MSG_WINDOW_LAYOUT.to_string(),
            pane_id: Some("@1".to_string()),
            layout: Some(layout.to_string()),
            ..Default::default()
        };
        let v = json(&m);
        assert_eq!(v["type"], "window.layout");
        assert_eq!(v["layout"], layout);
    }

    // Window move carries the +/-1 offset in amount.
    for offset in [-1, 1] {
        let m = WsIncoming {
            msg_type: MSG_WINDOW_MOVE.to_string(),
            pane_id: Some("@1".to_string()),
            amount: Some(offset),
            ..Default::default()
        };
        let v = json(&m);
        assert_eq!(v["amount"], offset);
    }

    // No pane.join exists: the known type list never contains it (D8 — logged
    // backend gap, never invented).
    let known = [
        MSG_PANE_SELECT,
        MSG_PANE_SPLIT,
        MSG_PANE_RESIZE,
        MSG_PANE_KILL,
        MSG_PANE_RENAME,
        MSG_PANE_ZOOM,
        MSG_PANE_BREAK,
        MSG_PANE_SWAP,
        MSG_WINDOW_SELECT,
        MSG_WINDOW_CREATE,
        MSG_WINDOW_RENAME,
        MSG_WINDOW_KILL,
        MSG_WINDOW_LAYOUT,
        MSG_WINDOW_MOVE,
        MSG_WINDOW_BREAK_ACTIVE,
    ];
    assert_eq!(known.len(), 15);
    assert!(!known.contains(&"pane.join"));
}
