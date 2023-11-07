#[inline]
pub(crate) fn display_text<S : AsRef<str>>(text: S) {
    let text = text.as_ref();
    let (columns, _) = crossterm::terminal::size().ok().unwrap_or((80, 25));
    let columns: usize = columns.into();
    let text = textwrap::wrap(text, columns);

    text.into_iter().for_each(|line| {
        println!("{}", line);
    });
}

macro_rules! display {
    ($text:expr) => {
        crate::io::display_text($text);
    };

    ($fmt:expr, $text:expr) => {
        crate::io::display_text(format!($fmt, $text));
    };
}

pub(crate) use display;
