use leptos::*;
use crate::core::rpc::RpcConnection;
use crate::pages::memo_card::MemoCard;

#[component]
pub fn HomePage() -> impl IntoView {
    let (burn_shard_data, set_burn_shard_data) = create_signal(String::new());
    
    // Create RPC connection and fetch latest burn shard data
    spawn_local(async move {
        let rpc = RpcConnection::new();
        match rpc.get_latest_burn_shard().await {
            Ok(result) => {
                set_burn_shard_data.set(result);
            },
            Err(e) => {
                set_burn_shard_data.set(format!("Error: {}", e));
            }
        }
    });

    view! {
        <div class="home-page">
            <h2>"Home"</h2>
            
            <div class="memo-cards">
                // Card 1
                <MemoCard
                    image="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==".to_string()
                    signature="5KN1ng2dSqZ3LBPgqyJVgBxnxwwBWAzm6wH7GNvQyAL4m5EUrNUCMz2hpC1w8mxDp1rof7rHyqX1KyqtZULmPmw".to_string()
                    pubkey="DuRBUwWoqMHwHiZVvQwz5FdZA4fKYxDBxqicdDVxpEZx".to_string()
                    blocktime=1709668246
                    amount=1.5
                />
                
                // Card 2
                <MemoCard
                    image="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==".to_string()
                    signature="7Jn2KqWxL8NzVYGR9qQP3vB1GQmDw5tX8JKrJu6YwF8sM4HdNcRzP9LmX2v6K8EqNVWjvZULhF3nKtQxPmCePmw".to_string()
                    pubkey="8xPzoZGqYKRgxqYJbJcXQm9xJqJwWLPx8J9fN6XNaEzt".to_string()
                    blocktime=1709668300
                    amount=2.3
                />
                
                // Card 3
                <MemoCard
                    image="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==".to_string()
                    signature="9pQ4R8vKmNgTX2YHsBz6wCt3nJLEuWqDx5VF2tPyB7MrK9HfQcWzL8NmY3v7K9FqNVXjvZULhF3nKtQxPmCePmw".to_string()
                    pubkey="3uVBXwWoqMHwHiZVvQwz5FdZA4fKYxDBxqicdDVxpEZx".to_string()
                    blocktime=1709668400
                    amount=0.8
                />
                
                // Card 4
                <MemoCard
                    image="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==".to_string()
                    signature="2Mn5KqWxL8NzVYGR9qQP3vB1GQmDw5tX8JKrJu6YwF8sM4HdNcRzP9LmX2v6K8EqNVWjvZULhF3nKtQxPmCePmw".to_string()
                    pubkey="5xPzoZGqYKRgxqYJbJcXQm9xJqJwWLPx8J9fN6XNaEzt".to_string()
                    blocktime=1709668500
                    amount=3.1
                />
                
                // Card 5
                <MemoCard
                    image="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==".to_string()
                    signature="4KN1ng2dSqZ3LBPgqyJVgBxnxwwBWAzm6wH7GNvQyAL4m5EUrNUCMz2hpC1w8mxDp1rof7rHyqX1KyqtZULmPmw".to_string()
                    pubkey="9uRBUwWoqMHwHiZVvQwz5FdZA4fKYxDBxqicdDVxpEZx".to_string()
                    blocktime=1709668600
                    amount=1.7
                />
                
                // Card 6
                <MemoCard
                    image="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==".to_string()
                    signature="6Jn2KqWxL8NzVYGR9qQP3vB1GQmDw5tX8JKrJu6YwF8sM4HdNcRzP9LmX2v6K8EqNVWjvZULhF3nKtQxPmCePmw".to_string()
                    pubkey="2xPzoZGqYKRgxqYJbJcXQm9xJqJwWLPx8J9fN6XNaEzt".to_string()
                    blocktime=1709668700
                    amount=2.8
                />
                
                // Card 7
                <MemoCard
                    image="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==".to_string()
                    signature="8pQ4R8vKmNgTX2YHsBz6wCt3nJLEuWqDx5VF2tPyB7MrK9HfQcWzL8NmY3v7K9FqNVXjvZULhF3nKtQxPmCePmw".to_string()
                    pubkey="7uVBXwWoqMHwHiZVvQwz5FdZA4fKYxDBxqicdDVxpEZx".to_string()
                    blocktime=1709668800
                    amount=1.2
                />
                
                // Card 8
                <MemoCard
                    image="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==".to_string()
                    signature="3Mn5KqWxL8NzVYGR9qQP3vB1GQmDw5tX8JKrJu6YwF8sM4HdNcRzP9LmX2v6K8EqNVWjvZULhF3nKtQxPmCePmw".to_string()
                    pubkey="4xPzoZGqYKRgxqYJbJcXQm9xJqJwWLPx8J9fN6XNaEzt".to_string()
                    blocktime=1709668900
                    amount=4.2
                />
                
                // Card 9
                <MemoCard
                    image="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==".to_string()
                    signature="1KN1ng2dSqZ3LBPgqyJVgBxnxwwBWAzm6wH7GNvQyAL4m5EUrNUCMz2hpC1w8mxDp1rof7rHyqX1KyqtZULmPmw".to_string()
                    pubkey="6uRBUwWoqMHwHiZVvQwz5FdZA4fKYxDBxqicdDVxpEZx".to_string()
                    blocktime=1709669000
                    amount=2.5
                />
                
                // Card 10
                <MemoCard
                    image="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==".to_string()
                    signature="0Jn2KqWxL8NzVYGR9qQP3vB1GQmDw5tX8JKrJu6YwF8sM4HdNcRzP9LmX2v6K8EqNVWjvZULhF3nKtQxPmCePmw".to_string()
                    pubkey="1xPzoZGqYKRgxqYJbJcXQm9xJqJwWLPx8J9fN6XNaEzt".to_string()
                    blocktime=1709669100
                    amount=3.7
                />
            </div>

            <div class="burn-shard-section">
                <h3>"Latest Burn Shard Data:"</h3>
                <pre>{burn_shard_data}</pre>
            </div>
        </div>
    }
} 