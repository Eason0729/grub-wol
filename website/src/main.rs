pub mod route;

use route::prelude::*;
use yew::prelude::*;
use yew_router::{BrowserRouter, Routable, Switch};

#[derive(Clone, Routable, PartialEq)]
enum Route {
    #[not_found]
    #[at("/login")]
    Login,
    #[at("/forbidden")]
    Forbidden,
    #[at("/control")]
    Control,
}

#[function_component(Forbidden)]
fn forbidden() -> Html {
    todo!()
}

fn switch(routes: Route) -> Html {
    match routes {
        Route::Login => html! {
            <Login />
        },
        Route::Forbidden => html! {
            <Forbidden />
        },
        Route::Control => html! {
            <Control />
        },
    }
}

#[function_component(Main)]
fn app() -> Html {
    html! {
        <BrowserRouter>
            <Switch<Route> render={switch} /> // <- must be child of <BrowserRouter>
        </BrowserRouter>
    }
}

fn main() {
    yew::Renderer::<Main>::new().render();
}
