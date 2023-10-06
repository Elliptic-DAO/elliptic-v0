import React, { createContext, useContext, useEffect, useState } from "react";
import { use_provide_auth, CORE_PRINCIPAL } from "./auth";
import { Account, AllowanceArgs, Allowance, Tokens } from "./interfaces/icrc1/icrc1";
import { Principal } from '@dfinity/principal';
import PropTypes from 'prop-types';
import { Vault, LiquidityStatus, ProtocolStatus } from './interfaces/core/core';

const E8S = 100_000_000;

export interface StateContext {
    active_component: string;
    set_active_component: (component_name: string) => void;
    refresh_balances: () => void;

    user_vaults: Vault[];
    vaults: Vault[];
    collateral_ratio: number;
    tvl: number;
    ckbtc_rate: number;
    total_liquidity_provided: number;

    liquidity_provided: number;
    liquidity_reward: number;
    liquidity_pool_share: number;

    ckbtc_allowance: number;
    ckbtc_balance: number;

    tal_allowance: number;
    tal_balance: number;

    is_loading: boolean;
    set_is_loading: (b: boolean) => void;
    is_logging_in: boolean;
    set_is_logging_in: (b: boolean) => void;
    handleImageLoad: () => void;
}

export function useProvideState(): StateContext {
    const auth = use_provide_auth();

    const [is_logging_in, set_is_logging_in_state] = useState(false);
    const [is_loading, set_is_loading_state] = useState(true);
    const [active_component, set_active_component] = useState('table-vaults');

    const [liquidity_reward, set_liquidity_reward] = useState(0);
    const [liquidity_provided, set_liquidity_provided] = useState(0);
    const [liquidity_pool_share, set_liquidity_pool_share] = useState(0);
    const [total_liquidity_provided, set_total_liquidity_provided] = useState(0);
    const [collateral_ratio, set_collateral_ratio] = useState(0);
    const [tvl, set_tvl] = useState(0);
    const [ckbtc_rate, set_ckbtc_rate] = useState(0);

    const [tal_allowance, set_tal_allowance] = useState(0);
    const [tal_balance, set_tal_balance] = useState(0);

    const [ckbtc_allowance, set_ckbtc_allowance] = useState(0);
    const [ckbtc_balance, set_ckbtc_balance] = useState(0);
    const [vaults, setVaults] = useState<Array<Vault>>([]);
    const [user_vaults, setUserVaults] = useState<Array<Vault>>([]);

    const handleImageLoad = () => {
        const allImagesLoaded = [...document.images].every(img => img.complete);
        set_is_loading_state(!allImagesLoaded);
        setTimeout(() => {
            set_is_loading_state(false);
        }, 1000);
    };

    function set_is_loading(b: boolean) {
        set_is_loading_state(b);
    }

    function set_is_logging_in(b: boolean) {
        set_is_logging_in_state(b);
    }

    async function fetch_protocol_status() {
        if (auth.core) {
            const protocol_status: ProtocolStatus = await auth.core.get_protocol_status();
            try {
                const tvl = Number(protocol_status.last_btc_rate) * Number(protocol_status.total_ckbtc_margin) / E8S;
                const cr = tvl / (Number(protocol_status.total_tal_borrowed) / E8S);
                set_ckbtc_rate(Number(protocol_status.last_btc_rate));
                set_tvl(tvl);
                set_collateral_ratio(cr);
            } catch (e) {
                console.error(e);
            }
        }
    }

    const fetchData = async (fetchFunc) => {
        let retries = 0;
        const retryLimit = 3;
        const retryDelay = 1000; // 1 second
        try {
            await fetchFunc();
        } catch (error) {
            if (retries < retryLimit) {
                retries++;
                await new Promise((resolve) => setTimeout(resolve, retryDelay));
                await fetchData(fetchFunc);
            } else {
                console.error(`Failed to fetch data after ${retries} retries: ${error}`);
            }
        }
    };

    useEffect(() => {
        if (auth.core) {
            const fetchAllData = async () => {
                await Promise.all([
                    fetchData(fetch_protocol_status),
                    fetchData(fetch_all_vaults),
                ]);
            };

            const intervalId = setInterval(fetchAllData, 10000); // fetch every 20 seconds

            fetchAllData(); // fetch data initially

            return () => {
                clearInterval(intervalId); // cleanup interval when component unmounts
            };
        }
    }, [auth.core]);

    useEffect(() => {
        if (auth.principal !== undefined) {
            set_active_component('vault');
        }
    }, [auth.principal]);

    async function fetch_ckbtc_balance() {
        if (auth.principal !== undefined && auth.ledger_ckbtc !== undefined) {
            const principal = Principal.fromText(auth.principal.toString());
            const user_account: Account = {
                'owner': principal,
                'subaccount': [],
            };
            const result: Tokens = await auth.ledger_ckbtc.icrc1_balance_of(user_account);
            console.log(result);
            const balance = parseInt(result.toString()) / Math.pow(10, 8);
            set_ckbtc_balance(balance)
        }
    }

    async function fetch_tal_balance() {
        if (auth.principal !== undefined && auth.ledger_tal !== undefined) {
            const principal = Principal.fromText(auth.principal.toString());
            const user_account: Account = {
                'owner': principal,
                'subaccount': [],
            };
            const result: Tokens = await auth.ledger_tal.icrc1_balance_of(user_account);
            const balance = parseInt(result.toString()) / Math.pow(10, 8);
            set_tal_balance(balance)
        }
    }

    async function fetch_all_vaults() {
        if (auth.core !== undefined) {
            const vaults: Array<Vault> = await auth.core.get_vaults([]);
            setVaults(vaults);
        }
    }

    async function fetch_user_vault() {
        if (auth.core !== undefined && auth.principal !== undefined) {
            const vaults: Array<Vault> = await auth.core.get_vaults([auth.principal]);
            setUserVaults(vaults);
        }
    }

    async function fetch_user_liquidity() {
        if (auth.core !== undefined && auth.principal !== undefined) {
            const status: LiquidityStatus = await auth.core.get_liquidity_status(auth.principal);
            set_liquidity_provided(Number(status.liquidity_provided) / E8S);
            set_liquidity_pool_share(Number(status.liquidity_pool_share));
            set_total_liquidity_provided(Number(status.total_liquidity_provided) / E8S);
            set_liquidity_reward(Number(status.available_liquidity_reward) / E8S);
        }
    }

    async function fetch_user_allowance_ckbtc() {
        if (auth.ledger_ckbtc !== undefined && auth.principal !== undefined) {
            const user_allowance: Allowance = await auth.ledger_ckbtc.icrc2_allowance({
                account: {
                    owner: auth.principal,
                    subaccount: [],
                } as Account, spender: {
                    owner: Principal.fromText(CORE_PRINCIPAL),
                    subaccount: [],
                } as Account

            } as AllowanceArgs);
            set_ckbtc_allowance(Number(user_allowance['allowance']));
        }
    }

    async function fetch_user_allowance_tal() {
        if (auth.ledger_tal !== undefined && auth.principal !== undefined) {
            const user_allowance: Allowance = await auth.ledger_tal.icrc2_allowance({
                account: {
                    owner: auth.principal,
                    subaccount: [],
                } as Account, spender: {
                    owner: Principal.fromText(CORE_PRINCIPAL),
                    subaccount: [],
                } as Account

            } as AllowanceArgs);
            set_tal_allowance(Number(user_allowance['allowance']));
        }
    }

    const refresh_balances = async () => {
        await Promise.all([
            fetchData(fetch_ckbtc_balance),
            fetchData(fetch_tal_balance),
            fetchData(fetch_user_vault),
            fetchData(fetch_user_allowance_ckbtc),
            fetchData(fetch_user_allowance_tal),
            fetchData(fetch_user_liquidity),
        ]);
    }

    useEffect(() => {
        if (auth.principal !== undefined) {
            const intervalId = setInterval(refresh_balances, 5000); // fetch every 5 seconds

            refresh_balances(); // fetch data initially

            return () => {
                clearInterval(intervalId); // cleanup interval when component unmounts
            };
        }
    }, [auth.principal, auth.ledger_ckbtc, auth.ledger_tal])


    useEffect(() => {
        handleImageLoad();
    }, []);

    return {
        is_logging_in,
        set_is_logging_in,

        is_loading,
        set_is_loading,

        active_component,
        set_active_component,

        handleImageLoad,

        liquidity_reward,
        collateral_ratio,
        liquidity_pool_share,
        liquidity_provided,
        total_liquidity_provided,

        tvl,
        ckbtc_rate,

        refresh_balances,
        ckbtc_allowance,
        ckbtc_balance,
        tal_balance,
        tal_allowance,

        user_vaults,
        vaults,
    }
}

const stateContext = createContext<StateContext>({} as StateContext);

ProvideState.propTypes = {
    children: PropTypes.node.isRequired,
};

export function ProvideState({ children }) {
    const state = useProvideState();
    return <stateContext.Provider value={state}> {children} </stateContext.Provider>;
}

export const use_local_state = () => {
    return useContext(stateContext);
};