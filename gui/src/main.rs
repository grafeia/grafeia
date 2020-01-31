use grafeia_app::app::App;
use pathfinder_view::show_pan;

fn main() {
    show_pan(App::load().unwrap_or_else(App::build));
}
