import React from "react";
import { use_local_state } from "../utils/state";

function convertToPercentage(number) {
    var percentage = (number * 100).toFixed(2) + '%';
    return percentage;
}

function convertNumber(num) {
    return Math.round((num / Math.pow(10, 8)) * 10) / 10;
}

export default function ProtocolStatus() {
    let state = use_local_state();
    return (
        <div style={{ border: 'solid', width: '40vw', marginLeft: '1em', padding: '1em' }}>
            <h3>Protocol Status</h3>
            <table style={{ marginLeft: '1em' }} className="status-table">
                <tbody>
                    <tr>
                        <td>Colateral Ratio</td>
                        <td>{convertToPercentage(state.collateral_ratio)}</td>
                    </tr>
                    <tr>
                        <td>TVL</td>
                        <td>${state.tvl.toFixed(2)}</td>
                    </tr>
                    <tr>
                        <td>ckBTC Price</td>
                        <td>${state.ckbtc_rate.toFixed(2)}</td>
                    </tr>
                    <tr>
                        <td>Covererable Amount</td>
                        <td>{convertNumber(state.coverable_amount)} ckBTC</td>
                    </tr>
                </tbody>
            </table>
            <h3>Canister Ids</h3>
            <table style={{ marginLeft: '1em' }} className="status-table">
                <tbody>
                    <tr>
                        <td>TAL</td>
                        <td>rkp4c-7iaaa-aaaaa-aaaca-cai</td>
                    </tr>
                    <tr>
                        <td>Core</td>
                        <td>rkp4c-7iaaa-aaaaa-aaaca-cai</td>
                    </tr>
                    <tr>
                        <td>Gauge</td>
                        <td>rkp4c-7iaaa-aaaaa-aaaca-cai</td>
                    </tr>
                </tbody>
            </table>
        </div>
    )
}