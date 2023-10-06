import "./my_vaults.css"
import React, { useState, useEffect } from "react";
import { use_local_state } from "../../utils/state";
import VaultMenu from "./VaultMenu";
import NewVault from "./NewVault";
import { display_ckbtc_e8s, display_tal_e8s } from "../lib";


export default function Vault() {
    const state = use_local_state();

    const [vault_index, setIndex] = useState(0);
    const [isVaultOpen, setVaultOpen] = useState(false);
    const [isAddOpen, setAddOpen] = useState(false);

    const handleCr = (vault) => {
        const value = (100 * Number(vault.ckbtc_margin_amount) * state.ckbtc_rate / Number(vault.borrowed_tal_amount)).toFixed(2);

        if (value === "Infinity") {
            return "∞"
        } else {
            return value + " %"
        }
    }

    const handleOpenMenu = (section, index) => {
        switch (section) {
            case "create":
                setAddOpen(true);
                break;

            case "vault":
                setVaultOpen(true);
                setIndex(index)
                break;
        }
    };

    const handleCloseVault = () => {
        setVaultOpen(false);
    };

    const handleCancelCreate = () => {
        setAddOpen(false);
    };

    const [moveRight, setDirection] = useState(true); // Direction of the animation

    function animateArrows() {
        const leftArrow = document.getElementById("leftArrow");
        const midArrow = document.getElementById("midArrow");
        const rightArrow = document.getElementById("rightArrow");

        if (leftArrow && rightArrow && midArrow) {
            const centerPosition = "translateX(0)"; // Initial position (centered)
            const moveAmountLeft = "40px"; // Adjust this value as needed
            const moveAmountMid = "20px";

            if (moveRight) {
                midArrow.style.transform = `translateX(${moveAmountMid})`;
                leftArrow.style.transform = `translateX(${moveAmountLeft})`;
            } else {
                leftArrow.style.transform = centerPosition;
                midArrow.style.transform = centerPosition;
            }
        }
    }

    useEffect(() => {
        // Call animateArrows immediately when the component mounts
        animateArrows();

        // Set up an interval to call animateArrows every 6 seconds
        const intervalId = setInterval(() => {
            animateArrows();
            setDirection(!moveRight);
        }, 1000);

        // Clean up the interval when the component unmounts
        return () => clearInterval(intervalId);
    }, [moveRight]); // The empty dependency array ensures this effect runs only once



    // Callback function to handle the intersection
    function handleOverFlow() {
        const vaultBox = document.getElementById('overflow');
        const arrows = document.getElementById('arrows-box');

        if (vaultBox && arrows) {
            if (vaultBox.scrollWidth <= vaultBox.clientWidth) {
                arrows.style.display = 'none';
            } else {
                arrows.style.display = 'flex';
            }
        }
    }
    handleOverFlow();



    const displayVaults = () => {
        return state.user_vaults.map(vault =>
            <div className="vault-box" key={Number(vault.vault_id)}>
                <div className="info-box">
                    <div className="name-box" style={{ display: "flex", justifyContent: "center", alignItems: "center" }}>
                        <span className="label" style={{ fontSize: "large" }}>{vault.vault_id.toString()}</span>
                    </div>
                    <div className="amount-box">
                        <div className="info">
                            <div style={{ marginLeft: 20 }}>
                                <span className="label">Margin:</span>
                            </div>
                            <div style={{ display: "flex", alignItems: "center", flexGrow: 1 }}>
                                <span className="value">{display_ckbtc_e8s(vault.ckbtc_margin_amount)} ckBTC</span>
                                <img
                                    alt="icon"
                                    src="/tokens/ckbtc_logo.svg"
                                    style={{ height: 30, margin: 10 }}
                                />
                            </div>
                        </div>
                        <div className="info">
                            <div style={{ marginLeft: 20 }}>
                                <span className="label">Borrow:</span>
                            </div>
                            <div style={{ display: "flex", alignItems: "center", flexGrow: 1 }}>
                                <span className="value">{display_tal_e8s(vault.borrowed_tal_amount)} TAL</span>
                                <img
                                    alt="icon"
                                    src="/tokens/taler_logo.png"
                                    style={{ height: 30, margin: 10 }}
                                />
                            </div>
                        </div>
                        <div className="info">
                            <div style={{ marginLeft: 20 }}>
                                <span className="label">Collateral Ratio:</span>
                            </div>
                            <div style={{ display: "flex", alignItems: "center" }}>
                                <span className="value">{handleCr(vault)}</span>
                            </div>
                        </div>
                    </div>
                </div>
                <div className="open-vault-box">
                    <button onClick={() => handleOpenMenu("vault", vault.vault_id)} className="open-vault-btn label" style={{ fontSize: "large" }}>Open vault</button>
                </div>
            </div>
        );
    };

    return (
        <>
            <div style={{ display: "flex", flexDirection: "column", width: "100%", justifyContent: "start" }}>
                {isAddOpen ? <NewVault onClose={handleCancelCreate} /> : <></>}
                {isVaultOpen ? <VaultMenu onClose={handleCloseVault} vault_id={BigInt(vault_index)} /> : <></>}
                <div
                    style={{
                        display: "flex",
                        height: "10vh",
                        alignItems: "center",
                        border: "solid 1px",
                        borderTop: 0,
                        borderLeft: 0,
                        borderRight: 0
                    }}
                >
                    <div
                        style={{
                            display: "flex",
                            justifyContent: "center",
                            alignItems: "center",
                            height: "80%",
                            width: 300
                        }}
                    >
                        <div
                            style={{
                                display: "flex",
                                alignItems: "center",
                                justifyContent: "center",
                                height: "80%",
                                width: "60%"
                            }}
                        >
                            <span style={{ fontWeight: "bolder" }}>My vaults</span>
                        </div>
                    </div>
                </div>
                <div id="overflow" className="scroll-bar" style={{ flexGrow: 1, display: "flex", marginTop: "5%", overflowX: 'scroll' }}>
                    <div className="create-box">
                        <button
                            style={{ width: '200px', height: '100%', border: 'none' }}
                            onClick={() => handleOpenMenu("create", null)}
                        >
                            <div
                                style={{
                                    display: "flex",
                                    flexDirection: "column",
                                    alignItems: "center",
                                    justifyContent: "center"
                                }}
                            >
                                <div style={{ display: "flex", alignItems: "center", justifyContent: "center" }}>
                                    <img alt="icon" src="/icon/plus_white.png" style={{ height: 40, width: 40 }} />
                                </div>
                                &nbsp;
                                <span className="label" style={{ fontSize: "large", color: "white" }}>Create vault</span>
                            </div>
                        </button>
                    </div>
                    {displayVaults()}
                </div>
                <div className="arrows" id="arrows-box">
                    <img className="arrows" id="leftArrow" src="/icon/right-arrow.png" alt="Right Arrow" />
                    <img className="arrows" id="midArrow" src="/icon/right-arrow-grey.png" alt="Right Arrow" />
                    <img className="arrows" id="rightArrow" src="/icon/right-arrow-white.png" alt="Right Arrow" />
                </div>
            </div>
        </>
    );
}