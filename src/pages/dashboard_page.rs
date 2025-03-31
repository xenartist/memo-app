use leptos::*;

#[component]
pub fn DashboardPage(
    version_status: ReadSignal<String>,
    blockhash_status: ReadSignal<String>
) -> impl IntoView {
    view! {
        <div class="dashboard-page">
            <h2>"Dashboard"</h2>
            <div class="rpc-status">
                <h3>"X1 RPC Status"</h3>
                <p>{version_status}</p>
                <p>{blockhash_status}</p>
            </div>
        </div>
    }
} 