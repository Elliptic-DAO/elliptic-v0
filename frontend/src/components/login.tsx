import { use_provide_auth } from "../utils/auth";
import { use_local_state } from "../utils/state";
import React from 'react';
import { toast } from "react-toastify";
import "./login.css";


enum Wallet {
    None,
    Bfinity,
    Plug,
}

export default function Login() {
    const auth = use_provide_auth();
    const state = use_local_state();

    async function connect_with_plug() {
        state.set_is_logging_in(false);
        state.set_is_loading(true);
        auth.connect_wallet(Wallet.Plug).then(() => {
            state.set_is_loading(false);
            toast.success('Welcome back!', {
                position: "top-right",
                autoClose: 5000,
                hideProgressBar: false,
                closeOnClick: true,
                pauseOnHover: true,
                draggable: true,
                progress: undefined,
                theme: "light",
            });
        })
    }

    async function connect_with_bfinity() {
        state.set_is_logging_in(false);
        state.set_is_loading(true);
        auth.connect_wallet(Wallet.Bfinity).then(() => {
            toast.success('Welcome back!', {
                position: "top-right",
                autoClose: 5000,
                hideProgressBar: false,
                closeOnClick: true,
                pauseOnHover: true,
                draggable: true,
                progress: undefined,
                theme: "light",
            });
            state.set_is_loading(false);
        })
    }

    return (
        <div style={{ display: 'flex', height: '100vh', alignItems: 'center', placeContent: 'center' }}>
            <div className="card">
                <div className="item item--1" onClick={connect_with_plug}>
                    <img src="/plug.svg" width='30px'></img>
                    <span className="text text--1"> Plug </span>
                </div>
                <div className="item item--2" onClick={connect_with_bfinity}>
                    <img src="/bitfinity.png" width='30px'></img>
                    <span className="text text--2"> Bfinity </span>
                </div>
                <div className="item item--3" onClick={() => state.set_is_logging_in(false)}>
                    {/* <svg height="24" width="24" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg"><path d="M0 0h24v24H0z" fill="none"></path><path fill="rgba(66,193,110,1)" d="M20.083 15.2l1.202.721a.5.5 0 0 1 0 .858l-8.77 5.262a1 1 0 0 1-1.03 0l-8.77-5.262a.5.5 0 0 1 0-.858l1.202-.721L12 20.05l8.083-4.85zm0-4.7l1.202.721a.5.5 0 0 1 0 .858L12 17.65l-9.285-5.571a.5.5 0 0 1 0-.858l1.202-.721L12 15.35l8.083-4.85zm-7.569-9.191l8.771 5.262a.5.5 0 0 1 0 .858L12 13 2.715 7.429a.5.5 0 0 1 0-.858l8.77-5.262a1 1 0 0 1 1.03 0zM12 3.332L5.887 7 12 10.668 18.113 7 12 3.332z"></path></svg>
                    <span className="quantity"> 150+ </span> */}
                    <span className="text text--3"> Close </span>
                </div>
                {/*
                <div className="item item--4">
                    <svg height="24" width="24" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg"><path d="M0 0h24v24H0z" fill="none"></path><path fill="rgba(220,91,183,1)" d="M12 20h8v2h-8C6.477 22 2 17.523 2 12S6.477 2 12 2s10 4.477 10 10a9.956 9.956 0 0 1-2 6h-2.708A8 8 0 1 0 12 20zm0-10a2 2 0 1 1 0-4 2 2 0 0 1 0 4zm-4 4a2 2 0 1 1 0-4 2 2 0 0 1 0 4zm8 0a2 2 0 1 1 0-4 2 2 0 0 1 0 4zm-4 4a2 2 0 1 1 0-4 2 2 0 0 1 0 4z"></path></svg>
                    <span className="quantity"> 30+ </span>
                    <span className="text text--4"> Animations </span>
                </div> */}
            </div>
            {/* <div style={{ display: 'flex', alignItems: 'center' }}>
                <img src="/bitfinity.png" width='30px'></img>
                <button onClick={connect_with_bfinity} style={{ fontWeight: 'bold' }}>
                    Bfinity Wallet
                </button>

            </div>
            <div style={{ display: 'flex', alignItems: 'center', marginTop: '1em' }}>
                <img src="/plug.svg" width='30px'></img>
                <button onClick={connect_with_plug} style={{ fontWeight: 'bold' }}>
                    Plug Wallet
                </button>
            </div> */}
        </div>
    );
}