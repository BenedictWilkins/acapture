use colored::Colorize;
use pyo3::prelude::*;

pub trait WarnOnErr<T> {
    fn unwrap_warn(self, py: Python) -> Option<T>;
}

// Implement the trait for any Result where the error type implements AsRef<str>
impl<T, E> WarnOnErr<T> for Result<T, E>
where
    E: AsRef<str>,
{
    fn unwrap_warn(self, py: Python) -> Option<T> {
        if let Err(e) = self {
            display_python_warning_local(e.as_ref());
            return None;
        } else {
            return self.ok();
        }
    }
}

fn display_python_warning_local(message: &str) {
    // TODO perhaps import and use the python warnings package?
    println!("{}", format!("WARNING: {}", message).yellow());
}

// Dummy function to simulate Python warning display
// fn display_python_warning(py: Python, message: &str) {
//     py.import("warnings")
//         .expect("Failed to import warnings module")
//         .getattr("warn")
//         .expect("Failed to get warn function")
//         .call1((message,))
//         .expect("Failed to issue warning");
// }
