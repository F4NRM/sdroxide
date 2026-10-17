//! **Look at the survey ground.**
//!
//! `the_painted_ground_puts_land_where_the_land_is` proves a place on the Earth
//! came out as land. It cannot prove the thing reads as a *map* — that is what
//! an eye is for, and the whole point of the change is that the ADS-B picture
//! stops being a stipple (r/ShortwavePlus, 2026-10-17: *"the map is dismal"*).
//!
//! A committed probe, not scratch: it draws the same image the air map does,
//! through the same code and the same palette, so the next person who changes
//! the ground can look at it before believing a test.
//!
//! Run it and open the files:
//!
//! ```text
//! cargo run --release -p sdroxide-ui --example survey_ground_probe
//! ```

use image::{Rgb, RgbImage};
use sdroxide_ui::ground_image;
use sdroxide_ui::theme;

fn main() {
    // Two views, because they are the two ways this can look wrong: a wide one
    // has to hold whole continents, a close one has to hold a coastline.
    let views: [(&str, (f64, f64, f64, f64)); 2] =
        [("europe", (50.0, 5.0, 60.0, 36.0)), ("north-sea", (54.0, 4.0, 12.0, 7.2))];
    for (name, view) in views {
        let path = format!("/tmp/opencode/ground-{name}.png");
        let img = ground_image(view, (900, 540), theme::map());
        let mut png = RgbImage::new(img.size[0] as u32, img.size[1] as u32);
        for (i, px) in img.pixels.iter().enumerate() {
            let (x, y) = (i as u32 % img.size[0] as u32, i as u32 / img.size[0] as u32);
            png.put_pixel(x, y, Rgb([px.r(), px.g(), px.b()]));
        }
        match png.save(&path) {
            Ok(()) => println!("wrote {path}"),
            Err(e) => println!("could not write {path}: {e}"),
        }
    }
    println!(
        "\nLook at them: land filled, sea the other colour, rivers and borders as \
         lines, coastline between texels rather than stepped along them."
    );
}
