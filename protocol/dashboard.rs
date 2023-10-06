use crate::read_state;
use std::io::Write;

pub fn build_dashboard() -> Vec<u8> {
    format!(
        "
    <!DOCTYPE html>
    <html lang=\"en\">
        <head>
            <title>Elliptic Protocol Dashboard</title>
            <style>
                table {{
                    border: solid;
                    text-align: left;
                    width: 100%;
                    border-width: thin;
                }}
                h3 {{
                    font-variant: small-caps;
                    margin-top: 30px;
                    margin-bottom: 5px;
                }}
                table table {{ font-size: small; }}
                .background {{ margin: 0; padding: 0; }}
                .content {{ max-width: 100vw; width: fit-content; margin: 0 auto; }}
                tbody tr:nth-child(odd) {{ background-color: #eeeeee; }}
            </style>
            <script>
                document.addEventListener(\"DOMContentLoaded\", function() {{
                    var tds = document.querySelectorAll(\".ts-class\");
                    for (var i = 0; i < tds.length; i++) {{
                    var td = tds[i];
                    var timestamp = td.textContent / 1000000;
                    var date = new Date(timestamp);
                    var options = {{
                        year: 'numeric',
                        month: 'short',
                        day: 'numeric',
                        hour: 'numeric',
                        minute: 'numeric',
                        second: 'numeric'
                    }};
                    td.title = td.textContent;
                    td.textContent = date.toLocaleString(undefined, options);
                    }}
                }});
            </script>
        </head>
        <body>
            <div class=\"background content\">
                <div>
                    <h3>Metadata</h3>
                    {}
                </div>
                <div>
                    <h3>Vault Table</h3>
                    <table>
                        <thead>
                            <tr>
                                <th>Vault Id</th>
                                <th>Owner</th>
                                <th>Borrowed TAL</th>
                                <th>ckBTC margin</th>
                            </tr>
                        </thead>
                        <tbody>{}</tbody>
                    </table>
                </div>
                <div>
                    <h3>Liquidity Table</h3>
                    <table>
                        <thead>
                            <tr>
                                <th>Owner</th>
                                <th>Amount</th>
                            </tr>
                        </thead>
                        <tbody>{}</tbody>
                    </table>
                </div>
                <div>
                    <h3>Liquidity Rewards</h3>
                    <table>
                        <thead>
                            <tr>
                                <th>Owner</th>
                                <th>Amount</th>
                            </tr>
                        </thead>
                        <tbody>{}</tbody>
                    </table>
                </div>
                <h3>Logs</h3>
                <table>
                    <thead>
                        <tr><th>Priority</th><th>Timestamp</th><th>Location</th><th>Message</th></tr>
                    </thead>
                    <tbody>
                        {}
                    </tbody>
                </table>
            </div>
        </body>
    </html>
    ",
        construct_metadata_table(),
        construct_vault_table(),
        construct_liquidity_table(),
        construct_liquidity_returns(),
        display_logs()
    )
    .into_bytes()
}

fn with_utf8_buffer(f: impl FnOnce(&mut Vec<u8>)) -> String {
    let mut buf = Vec::new();
    f(&mut buf);
    String::from_utf8(buf).unwrap()
}

fn construct_metadata_table() -> String {
    read_state(|s| {
        let last_btc_rate = s.last_btc_rate;
        let last_btc_timetsamp = s.last_btc_timestamp;
        format!(
            "<table>
                <tbody>
                    <tr>
                        <th>Mode</th>
                        <td>{}</td>
                    </tr>
                    <tr>
                        <th>TAL Ledger Principal</th>
                        <td>{}</td>
                    </tr>
                    <tr>
                        <th>ckBTC Ledger Principal</th>
                        <td>{}</td>
                    </tr>
                    <tr>
                        <th>XRC Principal</th>
                        <td>{}</td>
                    </tr>
                    <tr>
                        <th>BTC Rate</th>
                        <td>{}</td>
                    </tr>
                    <tr>
                        <th>Last BTC Price Timestamp</th>
                        <td class=\"ts-class\">{}</td>
                    </tr>
                    <tr>
                        <th>Total Collateral Ratio</th>
                        <td>{}%</td>
                    </tr>
                    <tr>
                        <th>Total circulating TAL</th>
                        <td>{}</td>
                    </tr>
                    <tr>
                        <th>Borrowing Fee</th>
                        <td>{}%</td>
                    </tr>
                    <tr>
                    <th>Last Redemption Fee</th>
                        <td>{}%</td>
                    </tr>
                    <tr>
                        <th>Vault Id</th>
                        <td>{}</td>
                    </tr>
                </tbody>
            </table>",
            s.mode,
            s.taler_ledger_principal,
            s.ckbtc_ledger_principal,
            s.xrc_principal,
            last_btc_rate.unwrap_or(crate::UsdBtc::from(rust_decimal::Decimal::ZERO)),
            last_btc_timetsamp.unwrap_or(0),
            s.total_collateral_ratio.to_f64() * 100.0,
            s.total_borrowed_tal_amount(),
            s.fee.to_f64() * 100.0,
            s.current_base_rate.to_f64() * 100.0,
            s.next_available_vault_id
        )
    })
}

fn construct_vault_table() -> String {
    with_utf8_buffer(|buf| {
        read_state(|s| {
            for (_vault_id, vault) in s.vault_id_to_vaults.iter() {
                write!(
                    buf,
                    "
                <tr>
                    <td>{}</td>
                    <td>{}</td>
                    <td>{}</td>
                    <td>{}</td>
                </tr>
                ",
                    vault.vault_id,
                    vault.owner,
                    (vault.borrowed_tal_amount),
                    (vault.ckbtc_margin_amount),
                )
                .unwrap();
            }
        });
    })
}

fn construct_liquidity_table() -> String {
    with_utf8_buffer(|buf| {
        read_state(|s| {
            for (principal, amount) in s.liquidity_pool.iter() {
                write!(
                    buf,
                    "
                <tr>
                    <td>{}</td>
                    <td>{}</td>
                </tr>
                ",
                    principal,
                    (*amount)
                )
                .unwrap();
            }
        });
    })
}

fn construct_liquidity_returns() -> String {
    with_utf8_buffer(|buf| {
        read_state(|s| {
            for (principal, amount) in s.liquidity_returns.iter() {
                write!(buf, "<tr><td>{}</td><td>{}</td></tr>", principal, (*amount)).unwrap();
            }
        })
    })
}

fn display_logs() -> String {
    use crate::logs::{DEBUG_BUF, INFO_BUF, TRACE_XRC_BUF};
    use ic_canister_log::{export, LogEntry};

    fn display_entry(buf: &mut Vec<u8>, tag: &str, e: &LogEntry) {
        write!(
            buf,
            "<tr><td>{}</td><td class=\"ts-class\">{}</td><td><code>{}:{}</code></td><td>{}</td></tr>",
            tag, e.timestamp, e.file, e.line, e.message
        )
        .unwrap()
    }

    let info = export(&INFO_BUF);
    let debug = export(&DEBUG_BUF);
    let trace_xrc = export(&TRACE_XRC_BUF);

    with_utf8_buffer(|buf| {
        for e in info.iter() {
            display_entry(buf, "info", e);
        }
        for e in debug.iter() {
            display_entry(buf, "debug", e);
        }
        for e in trace_xrc.iter() {
            display_entry(buf, "tracer_xrc", e);
        }
    })
}
