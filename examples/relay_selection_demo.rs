//! Demonstrates relay mode selection based on player count and preferences

use p2pgo_network::relay_provider::{PlayerGuild, RelayPreferences, RelayProviderFactory};

fn main() {
    println!("P2P Go - Relay Mode Selection Demo");
    println!("==================================\n");

    // Demonstrate relay selection for different player counts
    println!("1. Relay selection based on player count:");

    for player_count in 2..=5 {
        let provider = RelayProviderFactory::create_provider(
            player_count,
            vec![RelayPreferences::default(); player_count],
        );

        println!(
            "  {} players -> {} relay mode",
            player_count,
            provider.name()
        );
    }

    println!("\n2. Relay preferences based on player guilds:");

    // Show how different guilds affect relay preferences
    let guilds = [
        (PlayerGuild::Activity, "Aggressive"),
        (PlayerGuild::Reactivity, "Defensive"),
        (PlayerGuild::Avoidance, "Balanced"),
    ];

    for (guild, style) in guilds.iter() {
        println!(
            "\n  {} Guild ({} play style):",
            match guild {
                PlayerGuild::Activity => "Activity",
                PlayerGuild::Reactivity => "Reactivity",
                PlayerGuild::Avoidance => "Avoidance",
            },
            style
        );

        let pref = match guild {
            PlayerGuild::Activity => RelayPreferences {
                max_latency_ms: 50,
                prefer_direct: true,
                max_relay_hops: 1,
                guild: *guild,
            },
            PlayerGuild::Reactivity => RelayPreferences {
                max_latency_ms: 300,
                prefer_direct: false,
                max_relay_hops: 3,
                guild: *guild,
            },
            PlayerGuild::Avoidance => RelayPreferences::default(),
        };

        println!("    - Max latency: {}ms", pref.max_latency_ms);
        println!("    - Prefer direct: {}", pref.prefer_direct);
        println!("    - Max relay hops: {}", pref.max_relay_hops);
    }

    println!("\n3. Example game scenarios:");

    // Scenario 1: Quick 2-player game
    println!("\n  Scenario: Quick 2-player game between Activity guild members");
    let provider = RelayProviderFactory::create_for_guild(PlayerGuild::Activity, 2);
    println!(
        "    -> Using {} for low-latency direct connection",
        provider.name()
    );

    // Scenario 2: Stable 3-player game
    println!("\n  Scenario: 3-player game with mixed guilds");
    let preferences = vec![
        RelayPreferences {
            guild: PlayerGuild::Activity,
            ..Default::default()
        },
        RelayPreferences {
            guild: PlayerGuild::Reactivity,
            ..Default::default()
        },
        RelayPreferences {
            guild: PlayerGuild::Avoidance,
            ..Default::default()
        },
    ];
    let provider = RelayProviderFactory::create_provider(3, preferences);
    println!(
        "    -> Using {} for triangular relay with credit incentives",
        provider.name()
    );

    // Scenario 3: Large tournament
    println!("\n  Scenario: 8-player tournament");
    let provider = RelayProviderFactory::create_provider(8, vec![]);
    println!(
        "    -> Using {} for scalable multi-player support",
        provider.name()
    );

    println!("\n4. Network architecture benefits:");
    println!("  - True P2P: No central servers required");
    println!("  - Adaptive: Relay mode adjusts to player count");
    println!("  - Guild-aware: Respects player preferences");
    println!("  - Incentivized: Credit system rewards relay providers");
    println!("  - Resilient: Multiple relay strategies for reliability");
}
