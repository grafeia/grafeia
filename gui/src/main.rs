use grafeia_app::{
    app::App,
    view,
    view::Interactive
};

fn main() {
    env_logger::init();

    view::show(App::load().unwrap_or_else(App::build));
}
