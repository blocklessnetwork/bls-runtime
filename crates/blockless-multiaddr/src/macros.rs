macro_rules! byte_map {
    [$($flags:expr,)*] => {[
     $($flags != 0,)*
    ]};
}
