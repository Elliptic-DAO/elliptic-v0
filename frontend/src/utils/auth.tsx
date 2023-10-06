import React, { createContext, useContext, useEffect, useState } from "react";
import { HttpAgent, Actor, } from "@dfinity/agent";
import { _SERVICE as core_interface } from "./interfaces/core/core";
import { idlFactory as core_idl } from "./interfaces/core/core_idl";
import { _SERVICE as icrc1_interface } from "./interfaces/icrc1/icrc1";
import { idlFactory as icrc1_idl } from "./interfaces/icrc1/icrc1_idl";
import PropTypes from 'prop-types';
import canisterIds from './canister_ids.json';
import { Principal } from '@dfinity/principal';

export let LEDGER_CKBTC_PRINCIPAL: string, LEDGER_TAL_PRINCIPAL: string, CORE_PRINCIPAL: string;
if (process.env.REACT_APP_NETWORK == 'local') {
    LEDGER_CKBTC_PRINCIPAL = canisterIds.ckbtc_ledger.local;
    LEDGER_TAL_PRINCIPAL = canisterIds.taler_ledger.local;
    CORE_PRINCIPAL = canisterIds.core.local;
} else if (process.env.REACT_APP_NETWORK == 'ic') {
    LEDGER_CKBTC_PRINCIPAL = canisterIds.ckbtc_ledger.ic;
    LEDGER_TAL_PRINCIPAL = canisterIds.taler_ledger.ic;
    CORE_PRINCIPAL = canisterIds.core.ic;
}

export interface AuthContext {
    core?: core_interface,
    core_authentificated?: core_interface,
    ledger_ckbtc?: icrc1_interface,
    ledger_tal?: icrc1_interface,

    principal?: Principal,
    connect_wallet: (arg0: Wallet) => Promise<void>;
    disconnect_wallet: () => void;
}

export interface LoginWindow extends Window {
    ic: any;
}
declare let window: LoginWindow;

enum Wallet {
    None,
    Bfinity,
    Plug,
}

export function useProvideAuth(): AuthContext {
    const [core, set_core] = useState<core_interface | undefined>(undefined);
    const [core_authentificated, set_core_authentificated] = useState<core_interface | undefined>(undefined);

    const [ledger_ckbtc, set_ledger_ckbtc] = useState<icrc1_interface | undefined>(undefined);
    const [ledger_tal, set_ledger_tal] = useState<icrc1_interface | undefined>(undefined);

    const [principal, set_principal] = useState<Principal | undefined>(undefined);
    const [selected_wallet, set_selected_wallet] = useState<Wallet>(Wallet.None);

    const [host, set_host] = useState<string>("https://ic0.app");

    function disconnect_wallet() {
        set_principal(undefined);
        set_core_authentificated(undefined);

    }

    async function connect_wallet(wallet: Wallet) {
        let whitelist: string[] = [];
        whitelist = Object.values(canisterIds).map((entry) => entry.ic);

        // if (process.env.REACT_APP_NETWORK == 'local') {
        //     whitelist = Object.values(canisterIds).map((entry) => entry.local);
        //     set_host("http://localhost:8080/");
        // } else if (process.env.REACT_APP_NETWORK == 'ic') {
        //     set_host("https://ic0.app");
        // }
        const onConnectionUpdate = () => {
            console.log(window.ic.plug.sessionManager.sessionData)
        }
        switch (wallet) {
            case Wallet.Bfinity: {
                try {
                    await window.ic.infinityWallet.requestConnect({
                        whitelist,
                        host,
                        onConnectionUpdate,
                        timeout: 50000
                    });
                    set_selected_wallet(Wallet.Bfinity);
                    const bitfinity_principal = await window.ic.infinityWallet.getPrincipal();
                    set_principal(Principal.fromText(bitfinity_principal.toText()));
                } catch (e) {
                    console.error(e);
                }
                try {
                    const core_authentificated_actor_bitfinity = await window.ic.infinityWallet.createActor({
                        canisterId: CORE_PRINCIPAL,
                        interfaceFactory: core_idl,
                        host
                    });
                    set_core_authentificated(core_authentificated_actor_bitfinity);
                } catch (e) {
                    console.error(e);
                }
                try {
                    const ledger_ckbtc: icrc1_interface = await window.ic.infinityWallet.createActor({
                        canisterId: LEDGER_CKBTC_PRINCIPAL,
                        interfaceFactory: icrc1_idl,
                    });
                    set_ledger_ckbtc(ledger_ckbtc);

                    const ledger_TAL: icrc1_interface = await window.ic.infinityWallet.createActor({
                        canisterId: LEDGER_TAL_PRINCIPAL,
                        interfaceFactory: icrc1_idl,
                    });
                    set_ledger_tal(ledger_TAL);
                } catch (e) {
                    console.error(e);
                }
                break;
            }
            case Wallet.Plug: {
                try {
                    console.log(host)
                    await window.ic.plug.requestConnect({
                        whitelist,
                        host,
                        onConnectionUpdate,
                        timeout: 50000
                    });
                    const p = await window.ic.plug.getPrincipal();
                    set_principal(p);
                    set_selected_wallet(Wallet.Plug);
                    console.log(`The connected user's principal:`, p.toString());
                } catch (e) {
                    console.error(e);
                }
                try {
                    const core_authentificated_actor = await window.ic.plug.createActor({
                        canisterId: CORE_PRINCIPAL,
                        interfaceFactory: core_idl,
                    });
                    set_core_authentificated(core_authentificated_actor);

                    const ledger_ckbtc: icrc1_interface = await window.ic.plug.createActor({
                        canisterId: LEDGER_CKBTC_PRINCIPAL,
                        interfaceFactory: icrc1_idl,
                    });
                    set_ledger_ckbtc(ledger_ckbtc);

                    const ledger_TAL: icrc1_interface = await window.ic.plug.createActor({
                        canisterId: LEDGER_TAL_PRINCIPAL,
                        interfaceFactory: icrc1_idl,
                    });
                    set_ledger_tal(ledger_TAL);
                } catch (e) {
                    console.error(e);
                }
                break;
            }
        }
    }

    useEffect(() => {
        const initialize_actors = async () => {
            try {
                // if (process.env.REACT_APP_NETWORK == 'local') {
                //     set_host("http://localhost:8080/");
                // } else if (process.env.REACT_APP_NETWORK == 'ic') {
                //     set_host("https://ic0.app");
                // }
                const agent = new HttpAgent({
                    host
                });
                // agent.fetchRootKey();

                // if (process.env.REACT_APP_NETWORK == 'ic') {
                //     agent.fetchRootKey();

                // }

                const core_actor: core_interface = Actor.createActor(core_idl, { agent, canisterId: CORE_PRINCIPAL });
                set_core(core_actor);
            } catch (e) {
                console.error(e);
            }

        }
        initialize_actors();
    }, [])


    return {
        core,
        core_authentificated,
        ledger_ckbtc,
        ledger_tal,
        principal,
        connect_wallet,
        disconnect_wallet,
    };
}

const auth_context = createContext<AuthContext>({} as AuthContext);

export const use_provide_auth = () => {
    return useContext(auth_context);
}

export function ProvideAuth({ children }) {
    const auth = useProvideAuth();
    return <auth_context.Provider value={auth}>{children}</auth_context.Provider>;
}

ProvideAuth.propTypes = {
    children: PropTypes.node.isRequired,
};