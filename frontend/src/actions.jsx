import Vault from './components/vaults/my_vaults';
import Tablevaults from './components/vaults/vaults_table';
import { use_local_state } from './utils/state';
import Liquidity from './components/liquidity';
import Swap from './components/swap';

export default function Actions() {
  let state = use_local_state();

  return (
    <div className="container">
      {state.active_component === "vault" && <Vault />}
      {state.active_component === "swap" && <Swap />}
      {state.active_component === "table-vaults" && <Tablevaults />}
      {state.active_component === "liquidity" && <Liquidity />}
    </div>
  );
}