use yew::prelude::*;

#[function_component(App)]
fn app() -> Html {
    html! {
        <section style="font-family: system-ui, Arial, sans-serif; padding: 2rem;">
            <h1>{"Hello from Yew 👋"}</h1>
            <p>{"Se vedi questo messaggio nel browser, il setup WASM funziona."}</p>
        </section>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
