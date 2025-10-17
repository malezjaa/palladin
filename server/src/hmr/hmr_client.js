// Palladin HMR Client
(function() {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const host = window.location.host;
    const socketUrl = `${protocol}//${host}/__hmr`;
    
    let socket;
    let reconnectTimer;
    let isReconnecting = false;
    let isFirstConnection = true;

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
                if (isFirstConnection) {
                    console.log('[HMR] Ready');
                    isFirstConnection = false;
                } else {
                    console.log('[HMR] Server restarted, reloading page...');
                    window.location.reload();
                }
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
        const hasJsChanges = updates.some(update => {
            const path = update.path;
            return path.endsWith('.js') || path.endsWith('.jsx') || 
                   path.endsWith('.ts') || path.endsWith('.tsx');
        });

        const hasCssChanges = updates.some(update => update.path.endsWith('.css'));
        const hasHtmlChanges = updates.some(update => update.path.endsWith('.html'));

        if (hasCssChanges) {
            updates.forEach(update => {
                if (update.path.endsWith('.css')) {
                    reloadCSS(update.path);
                    console.log('[HMR] CSS hot reloaded:', update.path);
                }
            });
        }

        if (hasJsChanges || hasHtmlChanges) {
            console.log('[HMR] Module changed, reloading scripts...');
            reloadJavaScript();
        }
    }

    function reloadCSS(path) {
        const links = document.querySelectorAll('link[rel="stylesheet"]');
        let reloaded = false;
        
        links.forEach(link => {
            const href = link.getAttribute('href');
            if (href) {
                const url = new URL(link.href, window.location.origin);
                url.searchParams.set('t', Date.now().toString());
                link.href = url.toString();
                reloaded = true;
            }
        });

        return reloaded;
    }

    function reloadJavaScript() {
        const scripts = document.querySelectorAll('script[type="module"]');
        const scriptUrls = [];
        
        scripts.forEach(script => {
            if (script.src && !script.src.includes('__hmr')) {
                scriptUrls.push(script.src);
            }
        });

        if (scriptUrls.length === 0) {
            window.location.reload();
            return;
        }

        scripts.forEach(script => {
            if (script.src && !script.src.includes('__hmr')) {
                script.remove();
            }
        });

        scriptUrls.forEach(url => {
            const script = document.createElement('script');
            script.type = 'module';
            const newUrl = new URL(url);
            newUrl.searchParams.set('t', Date.now().toString());
            script.src = newUrl.toString();
            document.body.appendChild(script);
        });
    }

    // Connect when page loads
    connect();

    // Expose HMR API for future enhancements
    window.__PALLADIN_HMR__ = {
        socket,
        reconnect: connect
    };
})();
