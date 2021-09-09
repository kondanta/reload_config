/// This is a pretty small library for reloading changed config
/// without stop/starting the service.
/// Parameters and internals are pretty opinionated because I was
/// using this for a pet project.
mod recursive;

use anyhow::Result;
use crossbeam_channel::unbounded;
use notify::{
    RecommendedWatcher,
    Watcher,
};
use recursive::RecursiveMode;
use std::fmt::Debug;
use std::str::FromStr;
use std::sync::{
    Arc,
    Mutex,
};

/// A single function that handles the automatic config reload.
///
/// I've specifically put `C`onfig behind Arc<Mutex<>> but it would be better to
/// use something like Compare and swap.
///
/// Assume you have a config struct that is called Config
/// ```no_run
/// #[derive(serde::Deserialize)]
/// struct Config {
///    key: String,
/// }
/// // Now you have to implement your own loading config logic.
/// impl Config {
///   fn load() -> anyhow::Result<Config> {
///     let f = BufReader::new(File::open("./config.yaml")?);
///     serde_yaml::from_reader(f).map_err(|e| {
///       anyhow::anyhow!("Cannot parse the config file: {}",e.to_string())
///     })
///   }
/// }
///
/// // Create a config
/// let cfg = Arc::new(Mutex::new(Config::load()?));
/// let clone = Arc::clone(&cfg);
///
/// watch_changes::<Config>(
///   cfg,
///   "recursive".to_string(),
///   "/my/config/file/path/config.yaml".to_string()
///   Config::load,
/// );
pub fn watch_changes<C>(
    cfg: Arc<Mutex<C>>,
    mode: String,
    path: String,
    f: fn() -> Result<C>,
) -> Result<()>
where
    C: Debug + Send + 'static,
{
    let mode = RecursiveMode::from_str(&mode)?.convert();
    std::thread::spawn(move || loop {
        let (tx, rx) = unbounded();

        let mut watcher: RecommendedWatcher =
            RecommendedWatcher::new(tx).expect("Cannot create watcher");
        watcher
            .watch(std::path::Path::new(&path), mode)
            .expect("Cannot listen filesystem");

        match rx.recv().map_err(|e| {
            anyhow::anyhow!("Receiving events from watcher error: {:?}", e)
        }) {
            Ok(event) => match event {
                Ok(e) => {
                    tracing::trace!("Captured event: {:#?}", e);
                    // Modify: DataChange::Any gets triggered everytime you open
                    // the config file and perform empty save. It was kind of
                    // annoying so I had to use AccessKind:Close so that I can
                    // track if someone opened and closed the config file.
                    // However it does not track the changes the way Modify does
                    // so I go back to the modify.
                    // If it causes any undefined behaviours, consider changing
                    // back this to Accesskind again.
                    // Full code should be something like this:
                    // ```rust
                    // if e.kind == notify::EventKind::Access(AccessKind::Close(
                    //     AccessMode::Write,
                    // )) {...}
                    // ```
                    if e.kind.is_modify() ||
                        e.kind.is_create() ||
                        e.kind.is_remove()
                    {
                        tracing::trace!("Event kind: {:?}", e.kind);
                        match f() {
                            Ok(new_config) => {
                                tracing::trace!(
                                    "New config: {:?} - Old config: {:?}",
                                    &new_config,
                                    cfg,
                                );
                                *cfg.lock().unwrap() = new_config
                            }
                            Err(e) => {
                                tracing::error!("Cannot reload config: {:?}", e)
                            }
                        }
                    }
                }
                Err(e) => tracing::error!("Event error: {:?}", e),
            },
            Err(e) => tracing::error!("Error: {:?}", e),
        }
    });
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use anyhow::Result;
    use serde::Deserialize;
    use std::fs::File;
    use std::io::BufReader;
    use std::sync::{
        Arc,
        Mutex,
    };

    #[derive(Deserialize, Debug)]
    struct Config {
        test: String,
        val:  String,
    }

    impl Config {
        fn load() -> Result<Config> {
            let f = BufReader::new(File::open("./config.yaml")?);
            serde_yaml::from_reader(f).map_err(|e| {
                anyhow::anyhow!(
                    "Cannot parse the config file: {}",
                    e.to_string()
                )
            })
        }
    }
    #[test]
    fn reload_config() {
        let data = r#"---
test: "SomeData"
val: "SomeData"
        "#;
        std::fs::write("config.yaml", data).unwrap();
        let cfg = Arc::new(Mutex::new(Config::load().unwrap()));
        let clone = Arc::clone(&cfg);
        assert_eq!(clone.lock().unwrap().test, "SomeData");
        assert_eq!(clone.lock().unwrap().val, "SomeData");
        {
            let r = watch_changes(
                clone,
                "recursive".to_string(),
                "config.yaml".to_string(),
                Config::load,
            );
            assert_eq!(true, r.is_ok());
        }
        std::thread::sleep(std::time::Duration::from_secs(2));
        let data = r#"---
test: "OtherData"
val: "OtherData"
        "#;
        std::fs::write("config.yaml", data).unwrap();
        std::thread::sleep(std::time::Duration::from_secs(2));
        assert_eq!(cfg.lock().unwrap().test, "OtherData");
        assert_eq!(cfg.lock().unwrap().test, "OtherData");
    }
}
