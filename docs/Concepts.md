# Base concepts

Note: using ChatGPT to translate to English(About 70% of all?).

## Extensions

Each type of extension is defined by a relatively simple trait.

| Name                                       | Description                                                                                                                      |
| ------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------- |
| [Source](`crate::source::Source`)          | In terms of type, it is a `Stream<Item = Item>`. It is a data source.                                                            |
| [Generator](`crate::generator::Generator`) | It is similar to Source, but it takes an input and generates an arbitrary number of Items from it.                               |
| [Filter](`crate::filter::Filter`)          | It takes one Item (also called Context) along with an input from the user, and applies a predicate to decide whether to keep it. |
| [Sorter](`crate::sorter::Sorter`)          | It takes two Items and an input, and compares the Items with each other.                                                         |
| [UI](`crate::ui::UI`)                      | It takes input from the user, processes it and then displays it on the screen.                                                   |
| [Action](`crate::action::Action`)          | It takes the selected Item and executes the Action.                                                                              |

## Diagram

<!--
flowchart TD
    subgraph Source-like
    S[Source]
    G[Generator]
    end
    S[Source] --\>|Items| F[Filter]
    G[Generator] --\>|Items| F
    F --\> So[Sorter]
    So --\> U{UI}
    U --\> E[Action]
    U --\>|Input| So
    U --\>|Input| F
    U --\>|Input| G
-->

![image](https://github.com/user-attachments/assets/fc96d541-9123-4105-90d1-0bdb796a786e)

# Making the first launcher

Up to now, we have briefly explained the different types of extensions, but from here, we'll create a simple Launcher as a tutorial.
For a practical configuration, please refer to the [author's settings](https://github.com/satler-git/yurf).

On a side note: Even though it's called a `Launcher`, it can be used for purposes other than launching, so the name might be somewhat misleading.

## Create project

Create a binary crate with cargo. And add ltrait and tokio as a dependency.

```bash
cargo new hello-ltrait
cd hello-ltrait
cargo add ltrait
cargo add tokio --features=full
```

Configure error handler and logger:

```rust
use ltrait::color_eyre::Result;
use ltrait::{Launcher, Level};

#[tokio::main]
async fn main() -> Result<()> {
    // keeping _guard is required to write log
    let _guard = ltrait::setup(Level::INFO)?;
    // TODO: Configure and run Launcher

    Ok(())
}
```

## Create a launcher and set UI

I introduce the concept of Cusion.
Cusion is a type, and it is recommended to implement it using an `enum`.
Although it is not impossible to create a Launcher without Cusion, it is not recommended due to the significant limitations it imposes.
Transform the Items extracted from the Source or Generator into a Cusion. Then, from the Cusion, transform it into the Context for each Sorter, Filter, etc.

I recommend you to use [ltrait-ui-tui](https://github.com/ltrait/ui-tui) as your first ui.
Add as a dependency.

```bash
cargo add ltrait-ui-tui
```

And write `main.rs`.

```rust,ignore
use ltrait::color_eyre::Result;
use ltrait::{Launcher, Level};

use ltrait_ui_tui::{Tui, TuiConfig, TuiEntry, style::Style, Viewport};

enum Item {
    // TODO: add source
}


impl Into<String> for &Item {
    fn into(self) -> String {
        match self {
            // TODO:
            _ => "unknown item".into()
        }
    }
}


#[tokio::main]
async fn main() -> Result<()> {
    // keeping _guard is required to write log
    let _guard = ltrait::setup(Level::INFO)?;

    let launcher = Launcher::default()
        .set_ui(
            Tui::new(TuiConfig::new(
                Viewport::Fullscreen,
                '>', // Selected
                ' ',
                ltrait_ui_tui::sample_keyconfig,
            )),
            |c: &Item| TuiEntry {
                text: (c.into(), Style::new()),
            },
        );

    launcher.run().await
}
```

Since there is not even a single Source or Generator here, running it should result in just an input field. Let's try running it.

```bash
cargo run
```

## Add a source, filter, sorter

The simplest source can be created by [`crate::source::from_iter`].
You can add a source by [`crate::launcher::Launcher::add_source`].

If you only want to see even numbers, this can easily be achieved using [`crate::filter::ClosureFilter`] and [`crate::launcher::Launcher::add_raw_filter`].
And if you want to see the scene in order, you can use [`crate::sorter::ClosureSorter`] and [`crate::launcher::Launcher::add_raw_sorter`].

Performance can be optimised by setting the [`crate::launcher::Launcher::batch_size`].

Note: `add_raw_**` works like a syntax sugar. `add_raw_**(/* ... */)` will like
`add_**(/* ... */, |c| c)`. The behaviour changes a little due to lifetime and other factors, so it is recommended to use raw if a raw version can be used.

```rust,ignore
use ltrait::color_eyre::Result;
use ltrait::{
    Launcher,
    Level,
    filter::ClosureFilter,
    sorter::ClosureSorter,
};

use ltrait_ui_tui::{Tui, TuiConfig, TuiEntry, style::Style, Viewport};

use std::cmp;

enum Item {
    Num(u32)
}


impl Into<String> for &Item {
    fn into(self) -> String {
        match self {
            Item::Num(x) => format!("{x}"),
            _ => "unknown item".into()
        }
    }
}


#[tokio::main]
async fn main() -> Result<()> {
    // keeping _guard is required to write log
    let _guard = ltrait::setup(Level::INFO)?;

    let launcher = Launcher::default()
        // the simplest source
        .add_source(ltrait::source::from_iter(1..=5000), /* transformer */ Item::Num)
        .add_raw_filter(ClosureFilter::new(|c, _ /* input */| {
            match c {
                Item::Num(x) => (x % 2) == 0,
                _ => true, // If variants are added to Item in the future, they are ignored here
            }
        }))
        .reverse_sorter(false)
        .add_raw_sorter(ClosureSorter::new(|lhs, rhs, _| {
            match (lhs, rhs) {
                (Item::Num(lhs), Item::Num(rhs)) => lhs.cmp(rhs),
                _ => cmp::Ordering::Equal
            }
        }))
        .batch_size(500)
        .set_ui(
            Tui::new(TuiConfig::new(
                Viewport::Fullscreen,
                '>', // Selected
                ' ',
            )),
            |c| TuiEntry {
                text: (c.into(), Style::new()),
            },
        );

    launcher.run().await
}
```

Let's run it again.

```bash
cargo run
```

🎉 It's complete! 🎉

