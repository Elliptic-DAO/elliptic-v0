import "./VaultMenu.css"
import { use_local_state } from "../../utils/state";
import React, { useEffect, useState } from "react";
import { use_provide_auth, CORE_PRINCIPAL } from '../../utils/auth';
import { Account, ApproveArgs, ApproveResult } from '../../utils/interfaces/icrc1/icrc1';
import { VaultArg, Vault, Result, Result_1 } from '../../utils/interfaces/core/core';
import { Principal } from '@dfinity/principal';
import { toast } from "react-toastify";
import { display_ckbtc_e8s, display_tal_e8s, E8S, U64_MAX } from "../lib";

export default function VaultMenu(props: { onClose: () => void, vault_id: bigint }) {

    const state = use_local_state();
    const auth = use_provide_auth();

    const min_CR = 1.101;

    const vault: Vault = state.user_vaults.filter((vault) => { return vault.vault_id == props.vault_id; })[0];

    const max_deposit = state.ckbtc_balance;

    const maximum_borrowable_amount_left = (Number(vault.ckbtc_margin_amount) * state.ckbtc_rate) / min_CR - Number(vault.borrowed_tal_amount);

    //Handling collateral ratio
    const [cr, setCR] = useState(100 * Number(vault.ckbtc_margin_amount) * state.ckbtc_rate / Number(vault.borrowed_tal_amount));
    const updateCR = () => {
        const cr = 100 * Number(vault.ckbtc_margin_amount) * state.ckbtc_rate / Number(vault.borrowed_tal_amount);
        setCR(cr);
    }

    useEffect(() => {
        setCR(100 * Number(vault.ckbtc_margin_amount) * state.ckbtc_rate / Number(vault.borrowed_tal_amount));
        updateCR();
    }, [vault.borrowed_tal_amount, vault.ckbtc_margin_amount, state.ckbtc_rate]);
    //Handling deposit
    const [deposit_input, setDepositInput] = useState('0');

    //Handling borrow
    const [borrow_input, setBorrowInput] = useState('0');

    //Handling Pay off
    const [payOff_input, setPayOffInput] = useState('0');
    const [unlock_btn, setUnlockBtn] = useState('Pay-off')

    const handleCr = (vault) => {
        const value = (100 * Number(vault.ckbtc_margin_amount) * state.ckbtc_rate / Number(vault.borrowed_tal_amount)).toFixed(2);

        if (value === "Infinity") {
            return "∞"
        } else {
            return value + " %"
        }
    }

    const handleMax = (section) => {
        switch (section) {
            case "margin":
                setDepositInput(max_deposit.toFixed(2));
                break;

            case "borrow":
                setBorrowInput((maximum_borrowable_amount_left / E8S).toFixed(2));
                break;

            case "pay-off":
                if (vault.borrowed_tal_amount <= state.tal_balance * Math.pow(10, 8)) {
                    setPayOffInput(display_tal_e8s(vault.borrowed_tal_amount));
                    setUnlockBtn('Unlock Vault');
                } else {
                    setPayOffInput(state.tal_balance.toFixed(2));
                }
                break;
        }
    }

    const handleInputValue = (section, input) => {
        const number = parseFloat(input);
        const isNumber = /^[0-9]+(\.[0-9]{0,2})?$/;

        if (input === '' || isNumber.test(input)) {
            switch (section) {
                case "deposit":
                    if (isNaN(number) || number <= max_deposit) {
                        setDepositInput(input);
                    }
                    break;

                case "borrow":
                    if (isNaN(number) || number <= maximum_borrowable_amount_left) {
                        setBorrowInput(input);
                    }
                    break;

                case "pay-off":
                    if (isNaN(number) || number <= vault.borrowed_tal_amount) {
                        setPayOffInput(input);
                    }
                    break;
            }
        }
    }

    const handleDeposit = async () => {
        toast.success("handling deposit ...");
        const margin = parseFloat(deposit_input);
        if (!isNaN(margin)) {
            if (state.ckbtc_allowance < margin) {
                const spender = { owner: Principal.fromText(CORE_PRINCIPAL), subaccount: [] } as Account;
                if (auth.ledger_ckbtc === undefined) {
                    toast.error("Failed to interact with the protocol, please refresh the page.");
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
                }
            }
            if (auth.core_authentificated === undefined) {
                toast.error("Failed to interact with the protocol, please refresh the page.");
                return;
            }
            const result: Result_1 = await auth.core_authentificated.add_margin_to_vault({ 'vault_id': props.vault_id, 'amount': BigInt(margin * Math.pow(10, 8)) } as VaultArg);
            switch (Object.keys(result)[0]) {
                case 'Ok': {
                    toast.success("Successfully added margin to vault " + props.vault_id);
                    break;
                }
                case 'Err': {
                    toast.error("Failed to add margin: " + result['Err']);
                    props.onClose();
                    break;
                }
                default:
                    toast.error("Failed to interact with the protocol, please refresh the page.");
                    break;
            }
        }
    }

    const handleBorrow = async () => {
        toast.success("Handling borrow ...");
        const borrow_amount = parseFloat(borrow_input);
        if (!isNaN(borrow_amount)) {
            if (auth.core_authentificated === undefined) {
                toast.error("Failed to interact with the protocol, please refresh the page.");
                return;
            }
            const result: Result = await auth.core_authentificated.borrow_from_vault({ 'vault_id': props.vault_id, 'amount': BigInt(borrow_amount * Math.pow(10, 8)) } as VaultArg);
            switch (Object.keys(result)[0]) {
                case 'Ok': {
                    toast.success("Successfully borrowed from vault " + props.vault_id);
                    break;
                }
                case 'Err': {
                    toast.error("Failed to borrow: " + result['Err']);
                    props.onClose();
                    break;
                }
                default:
                    toast.error("Failed to interact with the protocol, please refresh the page.");
                    break;
            }
        }
    }

    const handlePayOff = async () => {
        toast.success("Handling pay-off ...");
        const payOff_value = parseFloat(payOff_input);
        if (!isNaN(payOff_value)) {
            if (state.tal_allowance < payOff_value || isNaN(state.tal_allowance)) {
                if (auth.ledger_tal === undefined) {
                    toast.error("Failed to interact with the protocol, please refresh the page.");
                    return;
                }
                const spender = { owner: Principal.fromText(CORE_PRINCIPAL), subaccount: [] } as Account;
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
                toast.success("Successfully approved Elliptic at block index: " + result_approve['Ok']);
            }
            if (auth.core_authentificated === undefined) {
                toast.error("Failed to interact with the protocol, please refresh the page.");
                return;
            }
            const result: Result_1 = await auth.core_authentificated.repay_to_vault({ 'vault_id': props.vault_id, 'amount': BigInt(payOff_value * Math.pow(10, 8)) } as VaultArg);
            switch (Object.keys(result)[0]) {
                case 'Ok': {
                    toast.success("Successfully repayed to vault " + props.vault_id);
                    break;
                }
                case 'Err': {
                    toast.error("Failed to repay: " + result['Err']);
                    props.onClose();
                    break;
                }
                default:
                    toast.error("Failed to interact with the protocol, please refresh the page.");
                    break;
            }
        }
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
                }}
            >
                <div
                    style={{
                        display: "flex",
                        flexDirection: "column",
                        width: "60%",
                        height: "40vw",
                        maxHeight: '800px',
                        maxWidth: '1100px',
                        border: "1px solid",
                        borderRadius: 15,
                        boxShadow: "rgba(0, 0, 0, 0.1) 8px 4px 8px",
                        backgroundColor: "#EEF1EF"
                    }}
                >
                    <div style={{ display: "flex", height: "30%" }}>
                        <div
                            style={{
                                display: "flex",
                                alignItems: "center",
                                justifyContent: "center",
                                width: "30%"
                            }}
                        >
                            <div
                                style={{
                                    height: "80%",
                                    width: "60%",
                                    display: "flex",
                                    justifyContent: "center",
                                    alignItems: "center"
                                }}
                            >
                                <span className="label" style={{ fontSize: "xx-large" }}>
                                    Vault Id: {vault.vault_id.toString()}
                                </span>
                            </div>
                        </div>
                        <div
                            style={{
                                display: "flex",
                                alignItems: "center",
                                justifyContent: "center",
                                width: "70%"
                            }}
                        >
                            <div
                                style={{
                                    height: "80%",
                                    width: "90%",
                                    display: "flex",
                                    justifyContent: "space-between",
                                    backgroundColor: "#d5dad7",
                                    borderRadius: 15
                                }}
                            >
                                <div className="columns-content">
                                    <div className="label-box">
                                        <div
                                            style={{
                                                display: "flex",
                                                justifyContent: "center",
                                                alignItems: "center"
                                            }}
                                        >
                                            <span className="label">Margin</span>
                                        </div>
                                    </div>
                                    <div
                                        style={{
                                            display: "flex",
                                            justifyContent: "center",
                                            alignItems: "center",
                                            flexGrow: 1
                                        }}
                                    >
                                        <div
                                            style={{
                                                display: "flex",
                                                justifyContent: "center",
                                                alignItems: "center"
                                            }}
                                        >
                                            <span>{display_ckbtc_e8s(vault.ckbtc_margin_amount)}</span>
                                        </div>
                                    </div>
                                </div>
                                <div className="columns-content">
                                    <div className="label-box">
                                        <div
                                            style={{
                                                display: "flex",
                                                justifyContent: "center",
                                                alignItems: "center"
                                            }}
                                        >
                                            <span className="label">Borrow</span>
                                        </div>
                                    </div>
                                    <div
                                        style={{
                                            display: "flex",
                                            justifyContent: "center",
                                            alignItems: "center",
                                            flexGrow: 1
                                        }}
                                    >
                                        <div
                                            style={{
                                                display: "flex",
                                                justifyContent: "center",
                                                alignItems: "center"
                                            }}
                                        >
                                            <span>{display_tal_e8s(vault.borrowed_tal_amount)}</span>
                                        </div>
                                    </div>
                                </div>
                                <div className="columns-content">
                                    <div className="label-box">
                                        <div
                                            style={{
                                                display: "flex",
                                                justifyContent: "center",
                                                alignItems: "center"
                                            }}
                                        >
                                            <span className="label">Collateral ratio</span>
                                        </div>
                                    </div>
                                    <div
                                        style={{
                                            display: "flex",
                                            justifyContent: "center",
                                            alignItems: "center",
                                            flexGrow: 1
                                        }}
                                    >
                                        <div
                                            style={{
                                                display: "flex",
                                                justifyContent: "center",
                                                alignItems: "center"
                                            }}
                                        >
                                            <span>{handleCr(vault)}</span>
                                        </div>
                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>
                    <div style={{ display: "flex", height: "5%", justifyContent: "end" }}>
                        <div
                            style={{
                                display: "flex",
                                justifyContent: "space-evenly",
                                marginRight: 20,
                                width: "10vw"
                            }}
                        >
                            <div
                                style={{
                                    display: "flex",
                                    justifyContent: "center",
                                    alignItems: "center"
                                }}
                            >
                                <span className="label">Transactions</span>
                            </div>
                            <div
                                style={{
                                    display: "flex",
                                    justifyContent: "center",
                                    alignItems: "center"
                                }}
                            >
                                <button style={{ border: "none" }}>
                                    <img alt="icon" src="/icon/plus.png" style={{ height: 20, width: 20 }} />
                                </button>
                            </div>
                        </div>
                    </div>
                    <div
                        style={{
                            display: "flex",
                            height: "50%",
                            justifyContent: "center",
                            alignItems: "center"
                        }}
                    >
                        <div
                            style={{
                                display: "flex",
                                height: "90%",
                                width: "90%",
                                borderRadius: 4,
                                justifyContent: "center",
                                alignItems: "center"
                            }}
                        >
                            <div
                                style={{
                                    display: "flex",
                                    flexDirection: "column",
                                    width: "90%",
                                    height: "90%",
                                    border: "1px solid",
                                    borderRadius: 8,
                                    borderColor: "#a3a2a2",
                                    backgroundColor: "#d5dad7",
                                    boxShadow: "rgba(0, 0, 0, 0.1) 8px 4px 8px"
                                }}
                            >
                                <div
                                    id="deposit"
                                    className="vault-action"
                                    style={{ borderTopLeftRadius: 8, borderTopRightRadius: 8 }}
                                >
                                    <div style={{ display: "flex", width: "90%", height: "100%" }}>
                                        <div className="token-box">
                                            <div className="input-box">
                                                <input className="token-input" onChange={(event) => handleInputValue("deposit", event.target.value)} value={deposit_input} />
                                                <div className="max-division">
                                                    <button onClick={() => handleMax("margin")} className="action-btn" style={{ width: 40 }}>
                                                        <span style={{ color: "white" }} className="label">
                                                            MAX
                                                        </span>
                                                    </button>
                                                </div>
                                                <div className="token-logo">
                                                    <img
                                                        alt="icon"
                                                        src="/tokens/ckbtc_logo.svg"
                                                        style={{ height: 40 }}
                                                    />
                                                </div>
                                            </div>
                                        </div>
                                    </div>
                                    <div className="validate-box">
                                        <button onClick={handleDeposit} className="action-btn">
                                            <span style={{ color: "white" }} className="label">
                                                Deposit
                                            </span>
                                        </button>
                                    </div>
                                </div>
                                <div
                                    id="borrow"
                                    className="vault-action"
                                    style={{ borderTopLeftRadius: 8, borderTopRightRadius: 8 }}
                                >
                                    <div style={{ display: "flex", width: "90%", height: "100%" }}>
                                        <div className="token-box">
                                            <div className="input-box">
                                                <input className="token-input" onChange={(event) => handleInputValue("borrow", event.target.value)} value={borrow_input} />
                                                <div className="max-division">
                                                    <button onClick={() => handleMax("borrow")} className="action-btn" style={{ width: 40 }}>
                                                        <span style={{ color: "white" }} className="label">
                                                            MAX
                                                        </span>
                                                    </button>
                                                </div>
                                                <div className="token-logo">
                                                    <img
                                                        alt="icon"
                                                        src="/tokens/taler_logo.png"
                                                        style={{ height: 40 }}
                                                    />
                                                </div>
                                            </div>
                                        </div>
                                    </div>
                                    <div className="validate-box">
                                        <button onClick={handleBorrow} className="action-btn">
                                            <span style={{ color: "white" }} className="label">
                                                Borrow
                                            </span>
                                        </button>
                                    </div>
                                </div>
                                <div
                                    className="vault-action"
                                    style={{ borderTopLeftRadius: 8, borderTopRightRadius: 8 }}
                                >
                                    <div style={{ display: "flex", width: "90%", height: "100%" }}>
                                        <div className="token-box">
                                            <div className="input-box">
                                                <input className="token-input" onChange={(event) => handleInputValue("pay-off", event.target.value)} value={payOff_input} />
                                                <div className="max-division">
                                                    <button onClick={() => handleMax("pay-off")} className="action-btn" style={{ width: 40 }}>
                                                        <span style={{ color: "white" }} className="label">
                                                            MAX
                                                        </span>
                                                    </button>
                                                </div>
                                                <div className="token-logo">
                                                    <img
                                                        alt="icon"
                                                        src="/tokens/taler_logo.png"
                                                        style={{ height: 40 }}
                                                    />
                                                </div>
                                            </div>
                                        </div>
                                    </div>
                                    <div className="validate-box">
                                        <button onClick={handlePayOff} className="action-btn">
                                            <span style={{ color: "white" }} className="label">
                                                {unlock_btn}
                                            </span>
                                        </button>
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
                            justifyContent: "center"
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
                                width: "50%",
                                height: "100%",
                                justifyContent: "center",
                                alignItems: "center",
                                margin: 30
                            }}
                        >

                            <button onClick={props.onClose} className="action-btn" style={{ height: "80%", width: "100%" }}>
                                <span style={{ color: "white" }} className="label">
                                    Close
                                </span>
                            </button>
                        </div>
                    </div>
                </div>
            </div>
        </>
    );
}