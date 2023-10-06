import './footer.css'

export default function Footer() {
    return (
        <footer>
            <div style={{ display: 'flex', flexDirection: 'row', alignItems: 'center' }}>
                <img src="/mobius_strip.png" className="icon" />
                <div style={{ display: 'flex' }}>
                    <h2 style={{ display: 'flex', alignItems: 'center' }}>Elliptic DAO</h2>
                    <span className="beta">beta</span>
                </div>
            </div>
            <div style={{ display: 'flex', flexDirection: 'row', alignItems: 'center' }}>
                <div style={{ display: 'flex', flexDirection: 'row', alignItems: 'center' }}>
                    <h2>Socials</h2>
                    <div className='links'>
                        <li><a href="https://twitter.com/elliptic_dao" target="_blank" rel="noreferrer"><p>Twitter</p></a></li>
                        <li><a href="https://oc.app/group/xtned-fqaaa-aaaar-atffa-cai/?ref=kkjx3-jyaaa-aaaar-atuia-cai&code=25dd9fe9c8bb6e35" target="_blank" rel="noreferrer"><p>Open Chat</p></a></li>
                    </div>
                </div>
                <div style={{ display: 'flex', flexDirection: 'row', alignItems: 'center' }}>
                    <h2>Ressources</h2>
                    <div className='links'>
                        <li><a href="https://github.com/Elliptic-DAO" target="_blank" rel="noreferrer"><p>Github</p></a></li>
                        <li><a href="https://elliptic-1.gitbook.io/elliptic-docs/" target="_blank" style={{ zIndex: 2 }}><p>Documentation↗</p></a></li>
                    </div>
                </div>
            </div>
        </footer >
    );
}