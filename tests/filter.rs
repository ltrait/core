use dummyui::DummyUI;
use ltrait::filter::ClosureFilter;
use ltrait::{Launcher, source::from_iter};
use std::convert::identity;
use std::sync::Arc;
use std::sync::Mutex;

mod dummyui;

const COUNT: i32 = 5000;

#[tokio::test]
async fn test_filter() -> Result<(), Box<dyn std::error::Error>> {
    let count = Arc::new(Mutex::new(0));
    let count_c = count.clone();
    let launcher = Launcher::default()
        .add_source(from_iter(0..COUNT), identity)
        .add_filter(ClosureFilter::new(|&x: &i32, _| (x % 2) == 0), |&c: &i32| c)
        .set_ui(
            DummyUI::new(|_: &i32| {
                *(*count).lock().unwrap() += 1;
            }),
            |&c: &i32| c,
        );

    launcher.run().await?;

    assert_eq!(*(*count_c).lock().unwrap(), COUNT / 2);

    Ok(())
}
