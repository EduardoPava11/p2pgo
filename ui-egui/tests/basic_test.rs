// SPDX-License-Identifier: MIT OR Apache-2.0

#[test]
fn basic_test() {
    println!("Basic test running");
    assert_eq!(2 + 2, 4);
}

#[test]
fn headless_feature_check() {
    #[cfg(feature = "headless")]
    println!("Headless feature is enabled!");

    #[cfg(not(feature = "headless"))]
    println!("Headless feature is NOT enabled!");
}
