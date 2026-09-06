//! Event handling for the terminal emulator.
//!
//! Bridges alacritty_terminal event system with GPUI via flume channels.

use alacritty_terminal::event::{Event, EventListener};

/// Events emitted by the terminal state machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalEvent {
    /// Terminal state updated; view needs redraw.
    Wakeup,
    /// Bell character received (visual or audible alert).
    Bell,
    /// Terminal title set via OSC escape sequence.
    Title(String),
    /// OSC 52 clipboard store request.
    ClipboardStore(String),
    /// OSC 52 clipboard load request.
    ClipboardLoad,
    /// Terminal child process exited.
    Exit,
}

/// An event proxy implementing Alacritty's EventListener trait.
#[derive(Clone)]
pub struct GpuiEventProxy {
    tx: flume::Sender<TerminalEvent>,
}

impl GpuiEventProxy {
    /// Create a new event proxy forwarding to a flume sender.
    pub fn new(tx: flume::Sender<TerminalEvent>) -> Self {
        Self { tx }
    }
}

impl EventListener for GpuiEventProxy {
    fn send_event(&self, event: Event) {
        let terminal_event = match event {
            Event::Wakeup => TerminalEvent::Wakeup,
            Event::Bell => TerminalEvent::Bell,
            Event::Title(title) => TerminalEvent::Title(title),
            Event::ResetTitle => TerminalEvent::Title(String::new()),
            Event::ClipboardStore(_, data) => TerminalEvent::ClipboardStore(data),
            Event::ClipboardLoad(_, _) => TerminalEvent::ClipboardLoad,
            Event::Exit | Event::ChildExit(_) => TerminalEvent::Exit,
            // Non-essential events ignored
            _ => return,
        };

        let _ = self.tx.send(terminal_event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_proxy_translation() {
        let (tx, rx) = flume::unbounded();
        let proxy = GpuiEventProxy::new(tx);

        proxy.send_event(Event::Wakeup);
        assert_eq!(rx.recv().unwrap(), TerminalEvent::Wakeup);

        proxy.send_event(Event::Title("my-shell".to_string()));
        assert_eq!(rx.recv().unwrap(), TerminalEvent::Title("my-shell".to_string()));

        proxy.send_event(Event::Bell);
        assert_eq!(rx.recv().unwrap(), TerminalEvent::Bell);
    }
}
