//! A backend for inline viewports that survives an unanswered cursor query.
//!
//! ratatui places an inline viewport from the cursor's row, which crossterm
//! reads with a cursor position report (`ESC[6n`) and gives up on after a
//! fixed two seconds. A terminal that is alive but too busy to answer in that
//! time (a starved emulator, a loaded host) would otherwise abort the prompt.
//! [`CursorFallbackBackend`] answers for it: the viewport is placed at the
//! bottom row, where a shell prompt usually leaves the cursor. When the
//! cursor was higher, the viewport draws lower than it, over rows that are
//! empty below the cursor. The late report reaches crossterm's event reader
//! while the prompt runs and is consumed there, not by the shell.

use ratatui::backend::{Backend, ClearType, WindowSize};
use ratatui::buffer::Cell;
use ratatui::layout::{Position, Size};

/// Delegates to `inner`, except that a failed cursor position read reports
/// the bottom row's first column instead of an error.
///
/// Implements only the base [`Backend`] trait: if the workspace enables
/// ratatui's `scrolling-regions` feature, its two scroll methods must be
/// delegated here too.
pub(super) struct CursorFallbackBackend<B> {
    inner: B,
}

impl<B: Backend> CursorFallbackBackend<B> {
    pub(super) fn new(inner: B) -> Self {
        Self { inner }
    }
}

impl<B: Backend> Backend for CursorFallbackBackend<B> {
    type Error = B::Error;

    fn draw<'a, I>(&mut self, content: I) -> Result<(), Self::Error>
    where
        I: Iterator<Item = (u16, u16, &'a Cell)>,
    {
        self.inner.draw(content)
    }

    fn append_lines(&mut self, n: u16) -> Result<(), Self::Error> {
        self.inner.append_lines(n)
    }

    fn hide_cursor(&mut self) -> Result<(), Self::Error> {
        self.inner.hide_cursor()
    }

    fn show_cursor(&mut self) -> Result<(), Self::Error> {
        self.inner.show_cursor()
    }

    fn get_cursor_position(&mut self) -> Result<Position, Self::Error> {
        match self.inner.get_cursor_position() {
            Ok(position) => Ok(position),
            Err(_) => {
                let size = self.inner.size()?;
                Ok(Position::new(0, size.height.saturating_sub(1)))
            }
        }
    }

    fn set_cursor_position<P: Into<Position>>(&mut self, position: P) -> Result<(), Self::Error> {
        self.inner.set_cursor_position(position)
    }

    fn clear(&mut self) -> Result<(), Self::Error> {
        self.inner.clear()
    }

    fn clear_region(&mut self, clear_type: ClearType) -> Result<(), Self::Error> {
        self.inner.clear_region(clear_type)
    }

    fn size(&self) -> Result<Size, Self::Error> {
        self.inner.size()
    }

    fn window_size(&mut self) -> Result<WindowSize, Self::Error> {
        self.inner.window_size()
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        self.inner.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::{Terminal, TerminalOptions, Viewport};

    /// A [`TestBackend`] whose cursor position read fails, like crossterm's
    /// when the terminal does not answer in time.
    struct Unanswered(TestBackend);

    impl Backend for Unanswered {
        type Error = std::io::Error;

        fn draw<'a, I>(&mut self, content: I) -> Result<(), Self::Error>
        where
            I: Iterator<Item = (u16, u16, &'a Cell)>,
        {
            ok(self.0.draw(content))
        }
        fn append_lines(&mut self, n: u16) -> Result<(), Self::Error> {
            ok(self.0.append_lines(n))
        }
        fn hide_cursor(&mut self) -> Result<(), Self::Error> {
            ok(self.0.hide_cursor())
        }
        fn show_cursor(&mut self) -> Result<(), Self::Error> {
            ok(self.0.show_cursor())
        }
        fn get_cursor_position(&mut self) -> Result<Position, Self::Error> {
            Err(std::io::Error::other("the cursor position could not be read within a normal duration"))
        }
        fn set_cursor_position<P: Into<Position>>(&mut self, position: P) -> Result<(), Self::Error> {
            ok(self.0.set_cursor_position(position))
        }
        fn clear(&mut self) -> Result<(), Self::Error> {
            ok(self.0.clear())
        }
        fn clear_region(&mut self, clear_type: ClearType) -> Result<(), Self::Error> {
            ok(self.0.clear_region(clear_type))
        }
        fn size(&self) -> Result<Size, Self::Error> {
            ok(self.0.size())
        }
        fn window_size(&mut self) -> Result<WindowSize, Self::Error> {
            ok(self.0.window_size())
        }
        fn flush(&mut self) -> Result<(), Self::Error> {
            ok(self.0.flush())
        }
    }

    fn ok<T>(result: Result<T, std::convert::Infallible>) -> std::io::Result<T> {
        result.map_err(|never| match never {})
    }

    #[test]
    fn an_unanswered_cursor_query_places_the_inline_viewport_at_the_bottom() {
        let unanswered = Unanswered(TestBackend::new(40, 20));
        let options = || TerminalOptions { viewport: Viewport::Inline(5) };
        assert!(Terminal::with_options(Unanswered(TestBackend::new(40, 20)), options()).is_err());

        let mut terminal = Terminal::with_options(CursorFallbackBackend::new(unanswered), options())
            .expect("the fallback answers for the terminal");
        let area = terminal.get_frame().area();
        assert_eq!((area.y, area.height), (15, 5), "the viewport's last row is the screen's");
    }

    #[test]
    fn an_answered_cursor_query_is_used_as_is() {
        let mut inner = TestBackend::new(40, 20);
        inner.set_cursor_position(Position::new(3, 2)).unwrap();
        let mut backend = CursorFallbackBackend::new(inner);
        assert_eq!(backend.get_cursor_position().unwrap(), Position::new(3, 2));
    }
}
