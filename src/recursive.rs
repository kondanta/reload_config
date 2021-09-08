/// List of available Recursive Modes
/// For more details, please check `notify` crate's RecursiveMode
pub enum RecursiveMode {
    Recursive,
    NonRecursive,
}

impl std::str::FromStr for RecursiveMode {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "recursive" => Ok(RecursiveMode::Recursive),
            "nonrecursive" => Ok(RecursiveMode::NonRecursive),
            _ => {
                tracing::error!("Cannot parse string: {}", s);
                Err(anyhow::anyhow!("Cannot parse string"))
            }
        }
    }
}

impl RecursiveMode {
    /// Converts RecursiveMode enum to `notify::RecursiveMode` type.
    ///
    /// ```no_run
    /// let rm: RecursiveMode = RecursiveMode::Recursive;
    /// let notify_type = rm.convert();
    /// assert_eq!(notify::RecursiveMode::Recursive, notify_type);
    /// ```
    pub(super) fn convert(&self) -> notify::RecursiveMode {
        match self {
            RecursiveMode::Recursive => notify::RecursiveMode::Recursive,
            RecursiveMode::NonRecursive => notify::RecursiveMode::NonRecursive,
        }
    }
}
