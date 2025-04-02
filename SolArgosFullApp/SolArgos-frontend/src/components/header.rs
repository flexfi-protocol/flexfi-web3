use leptos::prelude::*;
use leptos_router::components::A;

#[component]
pub fn Header() -> impl IntoView {
    view! {
        <header class="header glass-panel">
            <div class="container">
                <div class="header-content">
                    <A href="/">
                        <div class="logo-container">
                            <div class="logo-icon">
                                <svg class="logo-svg" viewBox="0 0 24 24" width="30" height="30">
                                    <path
                                        stroke="var(--secondary)"
                                        stroke-width="2"
                                        fill="none"
                                        d="M12,2 L2,7 L12,12 L22,7 L12,2 Z M2,17 L12,22 L22,17 M2,12 L12,17 L22,12"
                                    />
                                </svg>
                            </div>
                            <div class="logo-text">SolArgos</div>
                        </div>
                    </A>
                    
                    <nav class="main-nav">
                        <ul class="nav-list">
                            <li>
                                <A href="/">
                                    <span class="nav-link">Accueil</span>
                                </A>
                            </li>
                        </ul>
                    </nav>
                </div>
            </div>
        </header>
    }
}