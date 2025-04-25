pub mod item;

pub trait Generator {
    fn generate() -> Self;

    fn generate_many(number: usize) -> Vec<Self>
    where
        Self: Sized,
    {
        (0..number).map(|_| Self::generate()).collect()
    }
}
