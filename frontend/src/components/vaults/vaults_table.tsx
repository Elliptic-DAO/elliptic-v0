import "./vaults_table.css"
import React, { useEffect, useState } from "react";
import { use_local_state } from "../../utils/state";
import { display_principal } from "../lib";
import CopyButton from "./CopyButton";
import { display_ckbtc_e8s, display_tal_e8s } from "../lib";

export default function Tablevaults() {
    const state = use_local_state();

    const [vaults, setVaults] = useState(state.vaults);
    const [input, setInput] = useState('');

    useEffect(() => {
        handleInputChange(input);
    }, [input]);

    useEffect(() => {
        if (state.vaults.length > 0) {
            setVaults(state.vaults);
        }
    }, [state.vaults]);

    const handleVault = (vault) => {
        const value = (100 * Number(vault.ckbtc_margin_amount) * state.ckbtc_rate / Number(vault.borrowed_tal_amount)).toFixed(2);

        if (value === "Infinity") {
            return "∞"
        } else {
            return value + " %"
        }
    }


    const renderVaults = () => {
        return vaults.map(vault =>
            <tr key={Number(vault.vault_id)} className="rows">
                <td data-testid="id-cell" className="table-value cell">
                    {Number(vault.vault_id)}
                </td>
                <td data-testid="id-cell" className="table-value cell" style={{ display: 'flex' }}>
                    <p>
                        {display_principal(vault.owner.toString())}
                    </p>
                    <CopyButton textToCopy={vault.owner.toString()} />
                </td>
                <td data-testid="deposit-cell" className="table-value cell">
                    <span className="currency">{display_ckbtc_e8s(vault.ckbtc_margin_amount)}</span>
                </td>
                <td data-testid="borrow-cell" className="table-value cell">
                    <span className="currency">{display_tal_e8s(vault.borrowed_tal_amount)}</span>
                </td>
                <td data-testid="cr-cell" className="table-value cell">
                    <span className="currency">{handleVault(vault)}</span>
                </td>
            </tr>
        );
    };

    const handleClick = (category) => {
        const sortedItems = [...vaults];

        if (category === "deposit") {
            sortedItems.sort((vault1, vault2) => {
                if (vault1.ckbtc_margin_amount < vault2.ckbtc_margin_amount) {
                    return 1
                }
                if (vault1.ckbtc_margin_amount > vault2.ckbtc_margin_amount) {
                    return -1
                }
                return 0
            });
        }
        if (category === "borrow") {
            sortedItems.sort((vault1, vault2) => {
                if (vault1.borrowed_tal_amount < vault2.borrowed_tal_amount) {
                    return 1
                }
                if (vault1.borrowed_tal_amount > vault2.borrowed_tal_amount) {
                    return -1
                }
                return 0
            });
        }
        if (category === "cr") {
            sortedItems.sort((vault1, vault2) => {
                const cr1 = Number(vault1.ckbtc_margin_amount) * state.ckbtc_rate / Number(vault1.borrowed_tal_amount);
                const cr2 = Number(vault2.ckbtc_margin_amount) * state.ckbtc_rate / Number(vault2.borrowed_tal_amount);
                if (cr1 < cr2) {
                    return 1
                }
                if (cr1 > cr2) {
                    return -1
                }
                return 0
            });
        }
        setVaults(sortedItems);
    };

    const handleInputChange = (input) => {
        setInput(input);
        const filterItems = state.vaults.filter((vault) => {
            return input === vault.owner.toText().slice(0, input.length)
        });
        setVaults(filterItems);
    };

    return (
        <>
            <div className="table">
                <div className="table-banner">
                    <div className="table-name">All vaults</div>
                    <div className="filter">
                        <input
                            data-cy="vaults-search-input"
                            type="search"
                            placeholder="Choose Owner"
                            className="filter-bar"
                            onKeyDown={(event) => handleInputChange(event.currentTarget.value)}
                        />
                    </div>
                </div>
                <div style={{ overflowY: "scroll" }}>
                    <table className="table-data" style={{ border: 'none' }}>
                        <tbody style={{ borderSpacing: 10 }}>
                            <tr className="rows">
                                <th data-testid="id-cell" className="category">
                                    <span className="label">Vault Id</span>
                                </th>
                                <th data-testid="id-cell" className="category">
                                    <span className="label">Owner</span>
                                </th>
                                <th data-testid="deposit-cell" className="category">
                                    <div style={{ display: 'flex', justifyContent: 'space-between', placeContent: 'center' }}>
                                        <div style={{ display: "flex", alignItems: "center" }}>
                                            <span className="label">ckBTC margin</span>
                                            <img alt="icon" src="/tokens/ckbtc_logo.svg" style={{ height: 30, marginLeft: 10 }} />
                                        </div>
                                        <button className="filter-button" onClick={() => handleClick('deposit')}>
                                            <img src="/icon/sort_arrow.png" style={{ height: 16 }} />
                                        </button>
                                    </div>
                                </th>
                                <th data-testid="borrow-cell" className="category">
                                    <div style={{ display: 'flex', justifyContent: 'space-between', placeContent: 'center' }}>
                                        <div style={{ display: "flex", alignItems: "center" }}>
                                            <span className="label">Borrowed TAL</span>
                                            <img alt="icon" src="/tokens/taler_logo.png" style={{ height: 30, marginLeft: 10 }} />
                                        </div>
                                        <button className="filter-button" onClick={() => handleClick('borrow')}>
                                            <img src="/icon/sort_arrow.png" style={{ height: 16 }} />
                                        </button>
                                    </div>
                                </th>
                                <th data-testid="cr-cell" className="category">
                                    <div style={{ display: 'flex', justifyContent: 'space-between', placeContent: 'center' }}>
                                        <div style={{ display: 'flex', alignItems: 'center' }}>
                                            <span className="label">Collateral ratio</span>
                                        </div>
                                        <button className="filter-button" onClick={() => handleClick('cr')}>
                                            <img src="/icon/sort_arrow.png" style={{ height: 16 }} />
                                        </button>
                                    </div>
                                </th>
                            </tr>
                            {renderVaults()}
                        </tbody>
                    </table>
                </div>
            </div>
        </>
    );
}
