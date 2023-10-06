import React from 'react';
import PropTypes from 'prop-types';
import { Asset } from './lib';


SwapBox.propTypes = {
    changeFunction: PropTypes.func,
    value: PropTypes.number.isRequired,
    enableMax: PropTypes.bool,
    maximumFunction: PropTypes.func,
    fromAsset: PropTypes.any,
};

function SwapBox(props) {
    function get_token_url(asset: Asset) {
        switch (asset) {
            case Asset.ckBTC:
                return "/tokens/ckbtc_logo.svg"
            case Asset.TAL:
                return "/tokens/taler_logo.png"
        }
    }
    function assetToString(asset: Asset): string {
        switch (asset) {
            case Asset.ckBTC:
                return "ckBTC ";
            case Asset.TAL:
                return "TAL";
            default:
                throw new Error(`Unknown asset: ${asset}`);
        }
    }

    return (
        <div style={{ position: 'relative', border: 'solid', display: 'flex', alignItems: 'center', borderWidth: '3px', padding: '1em' }}>
            <input
                style={{
                    boxSizing: 'border-box',
                    border: 'none',
                    borderRadius: '4px',
                    paddingLeft: '8px',
                    width: '100%', // Set width to 100% to take up full space
                    height: '60px',
                    fontSize: '20px',
                }}
                pattern="^[0-9]*[.,]?[0-9]*$"
                autoCorrect="off"
                minLength={1}
                maxLength={79}
                onChange={(e) => props.changeFunction(e.target.value)}
                value={props.value}
            />
            <div style={{ display: 'flex', flexDirection: 'column' }}>
                <div style={{ display: 'flex' }}>
                    {props.enableMax ? <button style={{ padding: '5px', background: 'rgb(13, 124, 255)', color: 'white', border: 'none', boxShadow: '2px 3px 0 blue' }}
                        onClick={props.maximumFunction}
                    >
                        MAX
                    </button> : <></>}
                    <div style={{ display: 'flex', flexDirection: 'row-reverse', alignItems: 'center', border: 'solid', marginRight: '1em', padding: '5px', background: 'rgb(238, 241, 239)', borderWidth: '3px' }}>
                        <span style={{}}>
                            {assetToString(props.fromAsset)}
                        </span>
                        <img src={get_token_url(props.fromAsset)} width='40px'
                            style={{ borderRadius: '100%', marginRight: '5px' }}>
                        </img>
                    </div>
                </div>
            </div>
        </div>
    );
}

export default SwapBox;