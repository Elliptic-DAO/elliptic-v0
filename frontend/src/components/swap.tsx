import React, { useEffect, useState } from 'react';
import { use_local_state } from '../utils/state';
import { use_provide_auth, CORE_PRINCIPAL } from '../utils/auth';
import SwapBox from './swapBox';
import { Principal } from '@dfinity/principal';
import { Asset, TAL_TRANSFER_FEE } from './lib';
import { toast } from "react-toastify";
import { Result } from "../utils/interfaces/core/core";
import { Account, ApproveArgs, ApproveResult } from '../utils/interfaces/icrc1/icrc1';
import { U64_MAX } from "./lib";

function floatToBigInt(num: number): bigint {
    return BigInt(Math.round(num * 1e8));
}

export default function Swap() {
    const state = use_local_state();
    const auth = use_provide_auth();

    const from = Asset.TAL;
    const to = Asset.ckBTC;

    const [from_amount, set_from_amount] = useState<number>(100.000);
    const [from_amount_guard, set_from_amount_guard] = useState<number>(100.000);
    const [to_amount, set_to_amount] = useState<number>(0);
    const [isHovered, setIsHovered] = useState(false);

    const handleMouseEnter = () => {
        setIsHovered(true);
    };

    const handleMouseLeave = () => {
        setIsHovered(false);
    };

    async function make_transactions() {
        if (auth.ledger_tal === undefined || auth.principal === undefined) {
            return;
        }
        if (auth.principal !== undefined && !state.is_loading) {
            state.set_is_loading(true);
            const amount_to_swap = BigInt(floatToBigInt(from_amount));
            const spender: Account = { owner: Principal.fromText(CORE_PRINCIPAL), subaccount: [] };
            if (state.tal_allowance < amount_to_swap) {
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
                    toast.error("Failed to approve Elliptic.");
                    state.set_is_loading(false);
                    return;
                }
            }
            if (auth.core_authentificated !== undefined) {
                const result: Result = await auth.core_authentificated.redeem_ckbtc(amount_to_swap);
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
        }
        state.set_is_loading(false);
    }

    function compute_arrival_price(target) {
        set_to_amount(target * 1 / state.ckbtc_rate);
    }

    function set_maximum_from_amount() {
        if (state.tal_balance > TAL_TRANSFER_FEE / Math.pow(10, 8)) {
            set_from_amount(state.tal_balance - TAL_TRANSFER_FEE / Math.pow(10, 8));
            set_from_amount_guard(state.tal_balance - TAL_TRANSFER_FEE / Math.pow(10, 8));
        }
    }

    function change_from_amount(amount) {
        set_from_amount_guard(amount);
        try {
            set_from_amount(parseFloat(amount));
        } catch (e) {
            console.log(e);
        }
    }
    const rotationDegree = isHovered ? 180 : 0;

    useEffect(() => {
        set_from_amount(from_amount);
        compute_arrival_price(from_amount);
    }, [from_amount, from, state.ckbtc_rate])

    return (

        <div className='convert' style={{ display: 'flex', placeContent: 'center', flexDirection: 'column', border: 'solid', maxWidth: '480px', width: '100%', marginTop: '3em' }}>
            <div style={{ backgroundColor: '#EEF1EF', padding: '1em', boxShadow: '0px 1px 5px rgba(0, 0, 0, 0.2)' }}>
                <b>Swap</b>
            </div>
            <div style={{ alignItems: 'left', display: 'flex', flexDirection: 'column', padding: '1em' }}>
                <div>
                    <div style={{ position: 'relative' }}>
                        <SwapBox
                            fromAsset={from}
                            changeFunction={change_from_amount}
                            value={from_amount_guard}
                            maximumFunction={set_maximum_from_amount}
                            enableMax={true}
                        />
                    </div>
                </div>
                <div
                    style={{
                        transition: 'transform 0.5s ease',
                        transform: `rotate(${rotationDegree}deg)`,
                        transformOrigin: 'center',
                        width: '2em',
                        height: '2em',
                        alignSelf: 'center',
                        padding: '0.5em'
                    }}
                    onMouseEnter={handleMouseEnter}
                    onMouseLeave={handleMouseLeave}
                >
                    <img src="/icon/exchange.png" style={{ width: '100%', height: '100%' }} />
                </div>
                <div style={{ position: 'relative', }}>
                    <SwapBox
                        fromAsset={to}
                        value={to_amount}
                    />
                </div>
                {auth.principal ?
                    <button onClick={make_transactions} style={{ background: '#0D7CFF', color: 'white', border: 'none', marginTop: '2em', height: '55px', minWidth: '80px', position: 'relative', width: '100%', borderRadius: '10px', boxShadow: '2px 3px 0 blue' }}>
                        Swap
                    </button>
                    : <button onClick={() => state.set_is_logging_in(true)} style={{ background: '#0D7CFF', color: 'white', border: 'none', marginTop: '2em', height: '55px', minWidth: '80px', position: 'relative', width: '100%', borderRadius: '10px', boxShadow: '2px 3px 0 blue' }}>
                        <span>Connect your wallet</span>
                    </button>
                }
            </div>
        </div >
    );
}