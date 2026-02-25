/// Trait for spawning actors.
///
/// This is implemented for `tokio::runtime::Handle` and `tokio::runtime::Runtime`, allowing
/// them to be passed directly to `actor_with_executor`.
pub trait ActorSpawner {
    fn spawn_actor<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static;
}

impl ActorSpawner for tokio::runtime::Handle {
    fn spawn_actor<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.spawn(future);
    }
}

impl ActorSpawner for tokio::runtime::Runtime {
    fn spawn_actor<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.spawn(future);
    }
}
