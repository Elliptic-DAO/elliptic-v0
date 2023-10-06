import React from 'react';
import ProtocolStatus from './components/status';

function EllipticDescription() {
    return (
        <div style={{ maxWidth: '30vw' }}>
            <p> The Elliptic protocol is an innovative decentralized and over-collateralized stablecoin
                platform that aims to provide a stable and capital-efficient alternative to highly volatile cryptocurrencies.</p>
            <button style={{ border: 'solid', height: 'fit-content' }}>
                Whitepaper
            </button>
        </div>
    )
}


export default function EllipticStatus() {
    return (
        <div className="container">
            <EllipticDescription />
            <ProtocolStatus />
        </div>
    );
}