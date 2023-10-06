import EllipticStatus from '../elliptic_status';
import React, { useState, useEffect } from "react";
import { use_provide_auth } from "../utils/auth";
import { use_local_state } from '../utils/state';

export default function Analytics() {

    return (
        <div>
            <EllipticStatus />
        </div>
    );
}
