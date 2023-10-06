import { useState } from "react";
import { use_local_state } from "../utils/state";
import { use_provide_auth } from "../utils/auth";
import { display_principal } from "./lib";
import './navbar.css';

export default function Navbar() {
    let state = use_local_state();
    let auth = use_provide_auth();

    const [isHovered, setIsHovered] = useState(false);

    const handleMouseEnter = () => {
        setIsHovered(true);
    };

    const handleMouseLeave = () => {
        setIsHovered(false);
    };

    const handleClick = (value) => {
        if (auth.principal === undefined && value !== 'table-vaults' && value !== 'swap') {
            state.set_is_logging_in(true);
        } else {
            state.set_active_component(value);
        }
    };

    return (
        <nav style={{ boxShadow: '0px 1px 5px rgba(0, 0, 0, 0.2)' }}>
            <img src="/mobius_strip.png" onLoad={state.handleImageLoad} />
            <h1>Elliptic DAO</h1>
            <span className="beta">beta</span>
            <div>
                <div className={`navbar-links`}>
                    <button className="nav-button" onClick={() => handleClick('swap')}>
                        Swap
                    </button>
                    <button className="nav-button" onClick={() => handleClick('liquidity')}>
                        Liquidity
                    </button>
                    <button className="nav-button" onClick={() => handleClick('vault')}>
                        My Vaults
                    </button>
                    <button className="nav-button" onClick={() => handleClick('table-vaults')}>
                        All Vaults
                    </button>
                </div>
                {auth.principal === undefined ?
                    <button onClick={() => state.set_is_logging_in(true)}
                        className="nav-button over"
                        style={{
                            fontWeight: 'bold',
                        }}>
                        Connect
                    </button>
                    :
                    <div style={{ alignItems: 'center', display: 'flex', flexDirection: 'column', alignSelf: 'center' }}>
                        <button style={{ height: 'auto' }}
                            onMouseEnter={handleMouseEnter}
                            onMouseLeave={handleMouseLeave}
                            onClick={() => {
                                auth.disconnect_wallet();
                                setIsHovered(false);
                                handleClick('table-vaults');
                            }}
                        >
                            {isHovered ?
                                <div style={{ display: 'flex', flexDirection: 'column', width: '124px', height: '80px', placeContent: 'center' }}>
                                    <p>Disconnect</p>
                                </div>
                                :
                                <div style={{ display: 'flex', flexDirection: 'column', width: '124px', height: '80px', placeContent: 'center' }}>
                                    <b>{display_principal(auth.principal.toString())}</b>
                                    <p>{state.ckbtc_balance.toLocaleString(undefined, { minimumFractionDigits: 4, maximumFractionDigits: 4 })} ckBTC</p>
                                    <p>{state.tal_balance.toFixed(2)} TAL</p>
                                </div>
                            }

                        </button>
                    </div>
                }
            </div>

        </nav >
    );
}