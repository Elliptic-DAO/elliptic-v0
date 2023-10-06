import "./NewVault.css"
import React, { useState } from "react";
import { use_local_state } from "../../utils/state";
import { use_provide_auth, CORE_PRINCIPAL } from '../../utils/auth';
import { Account, ApproveArgs, ApproveResult } from '../../utils/interfaces/icrc1/icrc1';
import { VaultArg, OpenVaultResult, Result } from '../../utils/interfaces/core/core';
import { Principal } from '@dfinity/principal';
import { toast } from "react-toastify";
import { U64_MAX } from "../lib";

export default function NewVault(props: { onClose: () => void }) {

    const auth = use_provide_auth();
    const state = use_local_state();

    const min_CR = 1.101;

    const max_deposit = state.ckbtc_balance;

    //Handling deposit
    const [margin_input, setMarginInput] = useState('0');

    //Handling borrow
    const [borrow_input, setBorrowInput] = useState('0');

    //The maximum amount of Tal that can be borrowed from the amount ckBTC in the margin
    const max_borrow = (parseFloat(margin_input) * state.ckbtc_rate) / min_CR;

    const handleMax = (section) => {
        switch (section) {
            case "margin":
                setMarginInput(state.ckbtc_balance.toString());
                break;

            case "borrow": {
                const borrow_value = max_borrow;
                if (!isNaN(borrow_value)) {
                    setBorrowInput(borrow_value.toFixed(2));
                }
                break;
            }
        }
    }

    const handleInputValue = (section, input) => {
        const number = parseFloat(input);
        const isNumber = /^[0-9]+(\.[0-9]{0,8})?$/;

        if (input === '' || isNumber.test(input)) {
            switch (section) {
                case "margin":
                    setMarginInput(input);
                    break;

                case "borrow":
                    setBorrowInput(input);
                    break;
            }
        }
    }

    function defaultErrorMessage() {
        toast.error("Failed to interact with the protocol, please refresh the page.");
    }

    const handleCreation = async () => {
        const margin = parseFloat(margin_input);
        const borrow = parseFloat(borrow_input);

        if (isNaN(margin)) return;
        const spender = { owner: Principal.fromText(CORE_PRINCIPAL), subaccount: [] } as Account;
        if (margin < 0.001) {
            toast.warning("Minimum ckBTC margin is 0.001");
            state.set_is_loading(false);
            return;
        }
        state.set_is_loading(true);

        if (state.ckbtc_allowance < margin || isNaN(state.ckbtc_allowance)) {
            if (auth.ledger_ckbtc === undefined) {
                defaultErrorMessage();
                state.set_is_loading(false);
                props.onClose();
                return;
            }
            const result_approve: ApproveResult = await auth.ledger_ckbtc.icrc2_approve({
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
                state.set_is_loading(false);
            }
        }
        if (auth.core_authentificated === undefined) {
            defaultErrorMessage();
            props.onClose();
            state.set_is_loading(false);
            return;
        }
        toast.success("Opening new vault...");
        const result: OpenVaultResult = await auth.core_authentificated.open_vault(BigInt(margin * Math.pow(10, 8)));
        console.log(result);
        switch (Object.keys(result)[0]) {
            case 'Ok': {
                const vault_id = result['Ok']['vault_id'];
                toast.success("Successfully opened vault with id: " + vault_id);
                if (isNaN(borrow) || borrow === 0) {
                    state.set_is_loading(false);
                    props.onClose();
                    return;
                }
                if (borrow < 10) {
                    toast.warning("Minimum TAL borrow is 10");
                }
                const borrow_from_vault_result: Result = await auth.core_authentificated.borrow_from_vault({ vault_id, amount: BigInt(borrow * Math.pow(10, 8)) } as VaultArg);
                switch (Object.keys(borrow_from_vault_result)[0]) {
                    case 'Ok':
                        toast.success("Borrowed " + borrow + " TAL from vault");
                        props.onClose();
                        break;
                    case 'Err':
                        toast.error("Failed to borrow: " + result['Err']);
                        props.onClose();
                        break;
                    default:
                        break;
                }
                break;
            }
            case 'Err':
                toast.error("Failed to open vault: " + result.toString());
                props.onClose();
                break;
            default:
                toast.error("Unexpected result: " + result['Err']);
                props.onClose();
                break;
        }
        state.set_is_loading(false);
    }

    return (
        <>
            <div
                style={{
                    display: "flex",
                    justifyContent: "center",
                    alignItems: "center",
                    position: "absolute",
                    left: 0,
                    top: 0,
                    width: "100%",
                    height: "100%",
                    backdropFilter: "blur(10px)",
                    zIndex: 1,
                }}>
                <div
                    style={{
                        display: "flex",
                        flexDirection: "column",
                        width: "60%",
                        height: "30vw",
                        maxHeight: '700px',
                        maxWidth: '1000px',
                        border: "1px solid",
                        borderRadius: 15,
                        boxShadow: "rgba(0, 0, 0, 0.1) 8px 4px 8px",
                        backgroundColor: "#EEF1EF"
                    }}
                >
                    <div
                        style={{
                            display: "flex",
                            flexDirection: "column",
                            flexGrow: 1,
                            width: "100%",
                            borderRadius: 4,
                            justifyContent: "start",
                            alignItems: "center",
                            marginTop: 20
                        }}
                    >
                        <div
                            style={{
                                display: "flex",
                                flexDirection: "column",
                                width: "100%",
                                height: "100%",
                                border: "none",
                                backgroundColor: "transparent"
                            }}
                        >
                            <div
                                style={{
                                    display: "flex",
                                    alignItems: "center",
                                    justifyContent: "space-between",
                                    width: "100%",
                                    height: "25%"
                                }}
                            >
                                <div
                                    style={{
                                        height: "80%",
                                        width: "30%",
                                        display: "flex",
                                        justifyContent: "center",
                                        alignItems: "center",
                                        backgroundColor: "transparent",
                                        borderRadius: 8,
                                        border: "none"
                                    }}
                                >
                                    <span
                                        className="label"
                                        style={{ fontSize: "xx-large", color: "black" }}
                                    >
                                        New Vault
                                    </span>
                                </div>
                                <div
                                    style={{
                                        display: "flex",
                                        width: "50%",
                                        height: "100%",
                                        backgroundColor: "#d5dad7",
                                        borderRadius: 8,
                                        border: "none",
                                        alignItems: "center",
                                        justifyContent: "center",
                                        marginRight: 20
                                    }}
                                >
                                    <div
                                        style={{
                                            display: "flex",
                                            justifyContent: "center",
                                            alignItems: "center",
                                            height: "40%",
                                            width: "40%"
                                        }}
                                    >
                                        <span className="label" style={{ fontSize: "large" }}>
                                            Available tokens
                                        </span>
                                    </div>
                                    <div
                                        style={{
                                            display: "flex",
                                            justifyContent: "center",
                                            height: "60%",
                                            width: "60%",
                                            alignItems: "center"
                                        }}
                                    >
                                        <div style={{ display: "flex", height: "90%", width: "100%", justifyContent: "space-evenly" }}>
                                            <div className="user-token">
                                                <div className="token-value">
                                                    <span className="label" style={{ fontSize: "large" }}>
                                                        {state.ckbtc_balance.toLocaleString(undefined, { minimumFractionDigits: 4, maximumFractionDigits: 4 })}
                                                    </span>
                                                    &nbsp;
                                                    <img alt="icon" src="/tokens/ckbtc_logo.svg" style={{ height: 35 }} />
                                                </div>
                                            </div>
                                            <div className="user-token">
                                                <div className="token-value">
                                                    <span className="label" style={{ fontSize: "large" }}>
                                                        {state.tal_balance.toFixed(2)}
                                                    </span>
                                                    &nbsp;
                                                    <img alt="icon" src="/tokens/taler_logo.png" style={{ height: 35 }} />
                                                </div>
                                            </div>
                                        </div>
                                    </div>
                                </div>
                            </div>
                            <div
                                className="vault-action"
                                style={{ borderTopLeftRadius: 8, borderTopRightRadius: 8 }}
                            >
                                <div className="label-contouring-box">
                                    <div className="label-contouring">
                                        <span
                                            className="label"
                                            style={{ fontSize: "x-large", color: "black" }}
                                        >
                                            Deposit
                                        </span>
                                    </div>
                                </div>
                                <div style={{ display: "flex", flexGrow: 1, height: "60%" }}>
                                    <div className="token-box">
                                        <div className="input-box">
                                            <input className="token-input" value={margin_input} onChange={(event) => handleInputValue("margin", event.target.value)} />
                                            <div className="max-division">
                                                <button onClick={() => handleMax("margin")} className="action-btn" style={{ width: 40 }}>
                                                    <span style={{ color: "white" }} className="label">
                                                        MAX
                                                    </span>
                                                </button>
                                            </div>
                                            <div className="token-logo">
                                                <img alt="icon" src="/tokens/ckbtc_logo.svg" style={{ height: 40 }} />
                                            </div>
                                        </div>
                                    </div>
                                </div>
                            </div>
                            <div
                                className="vault-action"
                                style={{ borderTopLeftRadius: 8, borderTopRightRadius: 8 }}
                            >
                                <div className="label-contouring-box">
                                    <div className="label-contouring">
                                        <span
                                            className="label"
                                            style={{ fontSize: "x-large", color: "black" }}
                                        >
                                            Borrow
                                        </span>
                                    </div>
                                </div>
                                <div style={{ display: "flex", flexGrow: 1, height: "60%" }}>
                                    <div className="token-box">
                                        <div className="input-box">
                                            <input className="token-input" value={borrow_input} onChange={(event) => handleInputValue("borrow", event.target.value)} />
                                            <div className="max-division">
                                                <button onClick={() => handleMax("borrow")} className="action-btn" style={{ width: 40 }}>
                                                    <span style={{ color: "white" }} className="label">
                                                        MAX
                                                    </span>
                                                </button>
                                            </div>
                                            <div className="token-logo">
                                                <img alt="icon" src="/tokens/taler_logo.png" style={{ height: 40 }} />
                                            </div>
                                        </div>
                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>
                    <div
                        style={{
                            display: "flex",
                            height: "10%",
                            alignItems: "center",
                            justifyContent: "end",
                            marginBottom: 20
                        }}
                    >
                        <p style={{
                            fontStyle: 'italic',
                            marginLeft: '1em',
                            fontSize: '14px'
                        }}>
                            Please note that although the system is diligently tested, a hack or a bug that results in losses for the users can never be fully excluded.
                        </p>
                        <div
                            style={{
                                display: "flex",
                                width: "15%",
                                height: "100%",
                                justifyContent: "center",
                                alignItems: "center",
                                margin: 30
                            }}
                        >
                            <button
                                onClick={props.onClose}
                                className="btn-inverted"
                                style={{
                                    height: "80%",
                                    width: "100%",
                                }}
                            >
                                <span
                                    className="label"
                                >
                                    Cancel
                                </span>
                            </button>
                        </div>
                        <div
                            style={{
                                display: "flex",
                                width: "15%",
                                height: "100%",
                                justifyContent: "center",
                                alignItems: "center",
                                margin: 30
                            }}
                        >
                            <button onClick={handleCreation} className="action-btn" style={{ height: "80%", width: "100%" }}>
                                <span
                                    style={{ color: "white", backgroundColor: "rgb(13 124 255)" }}
                                    className="label"
                                >
                                    Create
                                </span>
                            </button>
                        </div>
                    </div>
                </div>

            </div>
        </>
    );
}