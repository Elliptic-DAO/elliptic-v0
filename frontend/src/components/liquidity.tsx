import React, { useState } from 'react';
import { use_local_state } from '../utils/state';
import { use_provide_auth, CORE_PRINCIPAL } from '../utils/auth';
import "./liquidity.css";
import { toast } from "react-toastify";
import { Account, ApproveArgs, ApproveResult } from '../utils/interfaces/icrc1/icrc1';
import { Result_1 } from "../utils/interfaces/core/core";
import { Principal } from '@dfinity/principal';

const U64_MAX = BigInt(18_446_744_073_709_551_615);

enum Operation {
    Add,
    Remove
}

export default function Liquidity() {
    const state = use_local_state();
    const auth = use_provide_auth();

    const [amount, set_amount] = useState<number>(1);
    const [guard_amount, set_guard_amount] = useState(1);

    function on_change_amount(amount) {
        try {
            set_amount(parseFloat(amount));
        } catch (e) {
            console.error(e);
        }
        set_guard_amount(amount);
    }

    async function make_liquidity_operation(operation_type: Operation) {
        if (auth.ledger_tal === undefined || auth.principal === undefined) {
            return;
        }
        if (!state.is_loading) {
            if (amount > 0 && !isNaN(amount)) {
                const amount_e8s = BigInt(amount * Math.pow(10, 8));
                state.set_is_loading(true);
                switch (operation_type) {
                    case Operation.Add: {
                        const spender: Account = { owner: Principal.fromText(CORE_PRINCIPAL), subaccount: [] };
                        if (state.tal_allowance < amount_e8s) {
                            const result_approve: ApproveResult = await auth.ledger_tal.icrc2_approve({
                                spender,
                                fee: [],
                                memo: [],
                                from_subaccount: [],
                                created_at_time: [],
                                expires_at: [],
                                expected_allowance: [],
                                amount: U64_MAX,
                            } as ApproveArgs);
                            if (Object.keys(result_approve)[0] === 'Ok') {
                                toast.success("Successfully approved Elliptic at block index: " + result_approve['Ok']);
                            } else {
                                toast.error("Failed to approve Elliptic.")
                            }
                        }
                        if (auth.core_authentificated !== undefined) {
                            const result: Result_1 = await auth.core_authentificated.provide_liquidity(amount_e8s);
                            switch (Object.keys(result)[0]) {
                                case 'Ok':
                                    toast.success("Successfully provided liquidity to Elliptic at block index: " + result['Ok']);
                                    break;
                                case 'Err':
                                    toast.error("Failed to provide liquidity: " + result['Err']);
                                    break;
                                default:
                                    toast.error("Failed to contact canister, please refresh the page.");
                                    break;
                            }
                        }
                        break;
                    }
                    case Operation.Remove: {
                        if (amount_e8s > state.liquidity_provided) {
                            if (auth.core_authentificated === undefined) {
                                toast.error("Failed to contact canister, please refresh the page.");
                                state.set_is_loading(false);
                                return;
                            }
                            const result: Result_1 = await auth.core_authentificated.withdraw_liquidity(amount_e8s);
                            switch (Object.keys(result)[0]) {
                                case 'Ok':
                                    toast.success("Successfully withdrawn liquidity from Elliptic at block index: " + result['Ok']);
                                    break;
                                case 'Err':
                                    toast.error("Failed to withdraw liquidity: " + result['Err']);
                                    break;
                                default:
                                    toast.error("Failed to contact canister, please refresh the page.");
                                    break;
                            }
                        }
                        break;
                    }
                }
                state.set_is_loading(false);
            }
        }
    }

    function set_maximum_add_liquidity() {
        set_guard_amount(state.tal_balance);
        set_amount(state.tal_balance);
    }

    return (
        <div
            style={{ display: "flex", justifyContent: "center", alignItems: "center", width: "100%", maxWidth: "1500px", marginTop: "10%" }}
        >
            <div
                style={{
                    display: "flex",
                    justifyContent: "space-evenly",
                    width: "80%",
                    alignItems: "center"
                }}
            >
                <div className="liquidity">
                    <div className="head-content">
                        <b>Manage Liquidity</b>
                    </div>
                    <div
                        style={{
                            height: "90%",
                            display: "flex",
                            flexDirection: "column",
                            justifyContent: "space-evenly",
                            padding: "1em"
                        }}
                    >
                        <div style={{ position: "relative" }}>
                            <input
                                style={{
                                    paddingRight: 64 /* width of image + some padding */,
                                    boxSizing: "border-box",
                                    border: "1px solid #ccc",
                                    borderRadius: 4,
                                    paddingLeft: 8,
                                    width: "100%",
                                    height: 50
                                }}

                                pattern="^[0-9]*[.,]?[0-9]*$"
                                autoCorrect="off"
                                minLength={1}
                                maxLength={79}
                                onChange={(e) => on_change_amount(e.target.value)}
                                value={guard_amount}
                            />
                            <button
                                style={{
                                    padding: 8,
                                    position: "absolute",
                                    right: 100,
                                    top: 10,
                                    height: 30
                                }}
                                className="btn"
                                onClick={set_maximum_add_liquidity}
                            >
                                MAX
                            </button>
                            <div
                                style={{
                                    position: "absolute",
                                    right: 20,
                                    top: 6,
                                    display: "flex",
                                    flexDirection: "row-reverse",
                                    alignItems: "center"
                                }}
                            >
                                <span>TAL</span>
                                <img
                                    src="/tokens/taler_logo.png"
                                    width="40px"
                                    style={{ borderRadius: "100%", marginRight: 5 }}
                                />
                            </div>
                        </div>
                        <p
                            style={{
                                marginTop: 3,
                                marginLeft: "auto",
                                display: "table",
                                marginBottom: "1em",
                                fontStyle: "italic"
                            }}
                        >
                            <span className="label">Liquidity Provided: </span>
                            <span>{state.liquidity_provided.toFixed(2)}</span>
                            <span> TAL</span>
                        </p>
                        <div style={{ display: "flex" }}>
                            <button
                                className="btn-inverted"
                                style={{ height: 55, minWidth: 80, marginLeft: "auto" }}
                                onClick={() => make_liquidity_operation(Operation.Remove)}
                            >
                                <span style={{ zIndex: 1 }}>Withdraw</span>
                            </button>
                            <button
                                className="btn"
                                style={{ height: 55, minWidth: 80, marginLeft: "1em" }}
                                onClick={() => make_liquidity_operation(Operation.Add)}
                            >
                                <span style={{ zIndex: 1 }}>Provide</span>
                            </button>
                        </div>
                    </div>
                </div>
                <div className="liquidity">
                    <div className="head-content">
                        <b>Liquidity rewards</b>
                    </div>
                    <div
                        style={{
                            display: "flex",
                            flexDirection: "column",
                            height: "80%",
                            justifyContent: "space-evenly",
                            paddingLeft: "2%"
                        }}
                    >
                        <p style={{ display: "flex", alignItems: "center" }}>
                            <span className="label">Liquidity Pool Share: </span>
                            <span>{(state.liquidity_pool_share * 100).toFixed(2)} %</span>
                        </p>
                        <p style={{ display: "flex", alignItems: "center" }}>
                            <span className="label">
                                Total liquidity provided to the protocol: </span>
                            <span>{state.total_liquidity_provided.toFixed(2)} </span>
                            <img
                                src="/tokens/taler_logo.png"
                                width="20px"
                                style={{ borderRadius: "100%", marginRight: 5 }}
                            />
                        </p>
                        <p style={{ display: "flex", alignItems: "center" }}>
                            <span className="label">Claimable Reward: </span>
                            <span>{state.liquidity_reward.toFixed(2)} </span>
                            <img
                                src="/tokens/ckbtc_logo.svg"
                                width="20px"
                                style={{ borderRadius: "100%", marginRight: 5 }}
                            />
                        </p>
                    </div>
                    <div style={{ display: "flex", height: "20%", justifyContent: "center" }}>
                        <button
                            style={{ margin: 0, height: "100%", minWidth: 80, width: "100%" }}
                            className="btn"
                        >
                            Claim Liquidity Reward
                        </button>
                    </div>
                </div>
            </div>
        </div>

    )
}