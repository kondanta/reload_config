# Config Reload

It is a small library for services that needs hot reload for their config 
files when they are updated without stop/starting the process.
Internal implementation may need serious refactor like freeing config from
`Arc<Mutex>>`. 
