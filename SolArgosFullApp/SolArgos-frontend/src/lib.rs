mod components;
mod pages;

use leptos::*;
use leptos::prelude::*;
use leptos_router::components::Router;
use components::header::Header;
use pages::home::HomePage;

#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <div class="app-container">
                <Header />
                <main>
                    <div>
                        <HomePage />
                    </div>
                </main>
                <footer class="footer">
                    <div class="container">
                        <p class="copyright">(c) 2023-2025 SolArgos. Propulsé par Helius SDK</p>
                    </div>
                </footer>
            </div>
        </Router>
    }
}

#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    mount_to_body(App);
}