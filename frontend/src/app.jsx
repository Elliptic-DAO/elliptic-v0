import Navbar from './components/navbar';
import Footer from './components/footer';
import Actions from './actions';
import InfoBar from './components/info_bar';
import SvgComponent from './components/landing';
import { use_local_state } from './utils/state';
import { ToastContainer } from "react-toastify";
import Login from "./components/login";
import 'react-toastify/dist/ReactToastify.css';


function App() {
  const isMobile = /iPhone|iPad|iPod|Android/i.test(navigator.userAgent);
  let state = use_local_state();


  if (isMobile) {
    return (
      <div>
        <SvgComponent />
      </div>
    );
  }

  return (
    <>
      {state.is_loading ?
        <div style={{ backgroundColor: 'rgba(0,0,0, 0.4)', position: 'absolute', width: '100vw', height: '100vh', zIndex: 5 }}>
          <div style={{ position: 'absolute', top: '50%', left: '50%', transform: 'translate(-50%, -50%)' }}>
            <img src="/mobius_strip.gif" style={{ order: '-1' }} />
          </div>
        </div>
        : null}
      {state.is_logging_in ?
        <div style={{ backgroundColor: 'rgba(0,0,0, 0.4)', position: 'absolute', width: '100vw', height: '100vh', zIndex: 5 }}>
          <Login />
        </div>
        : null}


      <>
        <div>
          <InfoBar />
          <Navbar />
        </div>
        <div>
          <ToastContainer
            position="top-right"
            autoClose={5000}
            hideProgressBar={false}
            newestOnTop={false}
            closeOnClick
            rtl={false}
            pauseOnFocusLoss
            draggable
            pauseOnHover
            theme="light"
          />
          {/* Same as */}
          <ToastContainer />
        </div>
        <div style={{ display: 'flex', flexDirection: 'column', minHeight: '80vh', height: '100%', width: '100%' }}>
          <Actions />
        </div>
        <Footer />
      </>

    </>
  );
}

export default App;