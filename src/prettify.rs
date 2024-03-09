pub const PRETTY_ALIGN: usize = 12;



#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct LeftAligned<'s>(pub &'s str);

impl std::fmt::Display for LeftAligned<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::fmt::Write;

        for _ in 0..PRETTY_ALIGN - self.0.len() {
            f.write_char(' ')?;
        }

        f.write_str(self.0)?;
        f.write_char(' ')?;

        Ok(())
    }
}



#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct RightAligned<'s>(pub &'s str);

impl std::fmt::Display for RightAligned<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::fmt::Write;

        f.write_str(self.0)?;
        f.write_char(' ')?;

        Ok(())
    }
}



pub fn print_aligned(left: &str, right: &str) -> std::io::Result<()> {
    use crossterm::{*, style::*};
    use std::io;

    execute! {
        io::stdout(),
        SetForegroundColor(Color::Green),
        Print(LeftAligned(left)),
        ResetColor,
        Print(RightAligned(right)),
        Print('\n'),
    }
}

pub fn error(msg: &str, help: Option<&str>) -> std::io::Result<()> {
    use crossterm::{*, style::*};
    use std::io;

    execute! {
        io::stdout(),
        SetForegroundColor(Color::Red),
        Print("error"),
        ResetColor,
        Print(": "),
        Print(msg),
        Print('\n'),
    }?;

    if let Some(help) = help {
        execute! {
            io::stdout(),
            Print('\n'),
            Print(help),
        }?;
    }

    Ok(())
}