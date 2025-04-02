use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::SubmitEvent;

use crate::components::transaction_view::{TransactionView, TransactionData};

#[component]
pub fn HomePage() -> impl IntoView {
    // État pour la signature de la transaction à analyser
    let (signature, set_signature) = signal(String::new());
    
    // État pour stocker la transaction récupérée
    let (transaction, set_transaction) = signal(None::<TransactionData>);
    
    // État pour gérer le chargement et les erreurs
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal(None::<String>);

    // Fonction pour récupérer les détails d'une transaction
    let fetch_transaction = move |sig: String| {
        set_loading.set(true);
        set_error.set(None);
        
        spawn_local(async move {
            let url = format!("/api/transaction/{}", sig);
            
            match reqwest::Client::new()
                .get(&url)
                .send()
                .await
            {
                Ok(response) => {
                    if response.status().is_success() {
                        match response.json::<serde_json::Value>().await {
                            Ok(json) => {
                                if let Some(data) = json.get("data") {
                                    if let (
                                        Some(signature), 
                                        Some(block_time), 
                                        Some(success), 
                                        Some(fee)
                                    ) = (
                                        data.get("signature").and_then(|v| v.as_str()), 
                                        data.get("block_time").and_then(|v| v.as_i64()),
                                        data.get("success").and_then(|v| v.as_bool()),
                                        data.get("fee").and_then(|v| v.as_u64())
                                    ) {
                                        let tx_type = data.get("transaction_type")
                                            .and_then(|v| v.as_str())
                                            .map(String::from);
                                        
                                        set_transaction.set(Some(TransactionData {
                                            signature: signature.to_string(),
                                            block_time,
                                            success,
                                            fee,
                                            transaction_type: tx_type,
                                        }));
                                    } else {
                                        set_error.set(Some("Format de transaction invalide".to_string()));
                                    }
                                } else {
                                    set_error.set(Some("Transaction non trouvée".to_string()));
                                }
                            },
                            Err(e) => set_error.set(Some(format!("Erreur de désérialisation: {}", e))),
                        }
                    } else {
                        set_error.set(Some(format!("Erreur HTTP: {}", response.status())));
                    }
                },
                Err(e) => set_error.set(Some(format!("Erreur de requête: {}", e))),
            }
            
            set_loading.set(false);
        });
    };

    // Gestionnaire pour la soumission du formulaire
    let handle_submit = move |ev: SubmitEvent| {
        ev.prevent_default();
        let sig = signature.get();
        if !sig.is_empty() {
            fetch_transaction(sig);
        }
    };

    view! {
        <div class="home-page">
            <div class="hero-section glass-panel">
                <div class="container">
                    <div class="hero-content">
                        <h1 class="hero-title">SolArgos</h1>
                        <p class="hero-description">
                            Explorateur de blockchain Solana avec analyse en temps réel
                        </p>
                    </div>
                </div>
            </div>

            <div class="container mt-lg">
                <div class="search-section card">
                    <h2>Analyser une transaction</h2>
                    <form on:submit=handle_submit>
                        <div class="form-group">
                            <label for="signature">Signature de transaction</label>
                            <input 
                                type="text"
                                id="signature"
                                placeholder="Entrez une signature de transaction Solana"
                                prop:value=signature
                                on:input=move |ev| set_signature.set(event_target_value(&ev))
                                required
                            />
                        </div>
                        <button type="submit" class="button button-primary">
                            Analyser
                        </button>
                    </form>
                </div>

                <div class="result-section mt-md">
                    {move || {
                        if loading.get() {
                            view! {
                                <div class="result-content">
                                    <div class="loading-spinner"></div>
                                    <p>Chargement de la transaction...</p>
                                </div>
                            }.into_any()
                        } else if let Some(err) = error.get() {
                            view! {
                                <div class="result-content">
                                    <div class="error-message">
                                        <p>{err}</p>
                                    </div>
                                </div>
                            }.into_any()
                        } else if let Some(tx) = transaction.get() {
                            view! {
                                <div class="result-content">
                                    <h2>Détails de la transaction</h2>
                                    <TransactionView transaction=tx />
                                </div>
                            }.into_any()
                        } else {
                            view! {
                                <div class="result-content">
                                    <h2>Aucune transaction</h2>
                                    <p>Entrez une signature pour voir les détails de la transaction</p>
                                </div>
                            }.into_any()
                        }
                    }}
                </div>
            </div>
        </div>
    }
}