import React, { useState, useEffect } from "react";

export default function SocialNetworks() {


    return (

        <div>
            <div className="connect">
                <button style={{ border: "none", padding: 5, borderRadius: "50%", marginLeft: "1em" }}>
                    <a href="http://www.twitter.com" target="_blank">
                        <img src="icon/twitter.png" alt="elliptic_twitter" width="30" height="30" />
                    </a>
                </button>
            </div>
            <div className="connect">
                <button style={{ border: "none", padding: 5, borderRadius: "50%", marginLeft: "1em" }}>
                    <a href="document/paper.txt" target="_blank">
                        <img src="icon/pdf.png" alt="" width="30" height="30" />
                    </a>
                </button>
            </div>
            <div className="connect">
                <button style={{ border: "none", padding: 5, borderRadius: "50%", marginLeft: "1em" }}>
                    <a href="https://www.medium.com" target="_blank">
                        <img src="icon/medium.png" alt="" width="30" height="30" />
                    </a>
                </button>
            </div>
        </div>
    );
}