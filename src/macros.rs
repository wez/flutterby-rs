#[macro_export]
#[cfg(feature = "simavr")]
macro_rules! logln {
    ($($args:tt)*) => ({
        simavr_logln!($($args)*);
    })
}

#[macro_export]
#[cfg(not(feature = "simavr"))]
macro_rules! logln {
    ($($args:tt)*) => ({})
}
