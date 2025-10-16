// Palladin HMR Client
(function() {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const host = window.location.host;
    const socketUrl = `${protocol}//${host}/__hmr`;
    
    let socket;
    let reconnectTimer;
    let isReconnecting = false;

    function connect() {
        socket = new WebSocket(socketUrl);

        socket.addEventListener('open', () => {
            console.log('[HMR] Connected to dev server');
            isReconnecting = false;
            if (reconnectTimer) {
                clearTimeout(reconnectTimer);
                reconnectTimer = null;
            }
        });

        socket.addEventListener('message', (event) => {
            try {
                const message = JSON.parse(event.data);
                handleMessage(message);
            } catch (error) {
                console.error('[HMR] Failed to parse message:', error);
            }
        });

        socket.addEventListener('close', () => {
            console.log('[HMR] Disconnected from dev server');
            if (!isReconnecting) {
                isReconnecting = true;
                reconnectTimer = setTimeout(() => {
                    console.log('[HMR] Attempting to reconnect...');
                    connect();
                }, 1000);
            }
        });

        socket.addEventListener('error', (error) => {
            console.error('[HMR] WebSocket error:', error);
        });
    }

    function handleMessage(message) {
        switch (message.type) {
            case 'connected':
                console.log('[HMR] Ready');
                break;

            case 'update':
                console.log('[HMR] Update received:', message.updates);
                handleUpdate(message.updates);
                break;

            case 'full-reload':
                console.log('[HMR] Full reload requested');
                window.location.reload();
                break;

            default:
                console.warn('[HMR] Unknown message type:', message.type);
        }
    }

    function handleUpdate(updates) {
        let needsReload = false;

        updates.forEach(update => {
            const path = update.path;

            if (path.endsWith('.css')) {
                // Hot reload CSS without page refresh
                if (reloadCSS(path)) {
                    console.log('[HMR] CSS hot reloaded:', path);
                } else {
                    needsReload = true;
                }
            } else if (path.endsWith('.js') || path.endsWith('.jsx') || path.endsWith('.ts') || path.endsWith('.tsx')) {
                // For JS/TS files, we need to reload
                // In the future, this could be enhanced with proper HMR
                needsReload = true;
            } else if (path.endsWith('.html')) {
                // HTML changes require a full reload
                needsReload = true;
            } else {
                // Other file types, reload to be safe
                needsReload = true;
            }
        });

        if (needsReload) {
            console.log('[HMR] Reloading page...');
            window.location.reload();
        }
    }

    function reloadCSS(path) {
        let updated = false;
        const links = document.querySelectorAll('link[rel="stylesheet"]');
        
        links.forEach(link => {
            const href = link.getAttribute('href');
            if (href && (href === path || href.includes(path.replace(/^\//, '')))) {
                const url = new URL(link.href, window.location.origin);
                url.searchParams.set('t', Date.now().toString());
                link.href = url.toString();
                updated = true;
            }
        });

        return updated;
    }

    // Connect when page loads
    connect();

    // Expose HMR API for future enhancements
    window.__PALLADIN_HMR__ = {
        socket,
        reconnect: connect
    };
})();
