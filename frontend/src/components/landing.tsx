import React, { FunctionComponent } from "react";
import "./landing.css"

const LandingPageMobile: FunctionComponent = () => {
    return (
        <div style={{ overflow: 'hidden' }}>

            <section className="container-mobile">
                <img className="elliptic-logo" alt="" src="landing/logo.png" />
                <div className="section">
                    <div className="title">
                        <h1>Elliptic</h1>
                        <h1>Protocol</h1>
                    </div>
                    <div className="sub-title">
                        <p>A decentralized stablecoin,</p>
                        <p>backed by ckBTC</p>
                    </div>
                    <div className="social-images">
                        <a href="https://twitter.com/elliptic_dao" target="_blank">
                            <img alt="" src="landing/twitter.png" />
                        </a>
                        <a href="https://oc.app/group/xtned-fqaaa-aaaar-atffa-cai/?ref=kkjx3-jyaaa-aaaar-atuia-cai&code=25dd9fe9c8bb6e35" target="_blank">
                            <img alt="" src="landing/open_chat.png" />
                        </a>
                    </div>
                </div>
                <button className="doc-button">
                    <a href="https://elliptic-1.gitbook.io/elliptic-docs/" target="_blank" style={{ zIndex: 2, color: 'white' }}>
                        Documentation↗
                    </a>
                </button>
                <p style={{ fontStyle: 'italic', marginTop: '5em', color: 'grey', position: 'absolute', top: '100vh' }}>The dapp is not yet supported on mobile.</p>
            </section>
            <img
                className="image"
                style={{ filter: 'blur(12px)' }}
                alt=""
                src="tokens/taler_logo.png"
            />
            <div className="link-landing">
                <a href="https://twitter.com/elliptic_dao" target="_blank" rel="noreferrer"><p>Twitter</p></a>
                <a href="https://oc.app/group/xtned-fqaaa-aaaar-atffa-cai/?ref=kkjx3-jyaaa-aaaar-atuia-cai&code=25dd9fe9c8bb6e35" target="_blank" rel="noreferrer"><p>Open Chat</p></a>
                <a href="https://github.com/Elliptic-DAO" target="_blank" rel="noreferrer"><p>Github</p></a>
                <a href="https://elliptic-1.gitbook.io/elliptic-docs/" target="_blank" style={{ zIndex: 2 }}><p>Documentation</p></a>
            </div>
            <p className='img-icp'><img alt="" src="landing\icp_powered_by.svg" /></p>
        </div>
    );
};

export default LandingPageMobile;