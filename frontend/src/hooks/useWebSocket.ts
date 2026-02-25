import { useEffect, useRef, useCallback, useState } from 'react';
import { useChatStore } from '../stores/appStore';
import { useBackendStore } from '../stores/backendStore';
import type { CompanyEvent, Message } from '../types';

export function useWebSocket() {
  const ws = useRef<WebSocket | null>(null);
  const reconnectTimeout = useRef<ReturnType<typeof setTimeout>>();
  const reconnectAttempts = useRef(0);
  const maxReconnectAttempts = 5;
  
  const [connected, setConnected] = useState(false);
  const { addMessage, setTyping, updateSession } = useChatStore();

  const connect = useCallback(() => {
    if (ws.current?.readyState === WebSocket.OPEN) return;

    // Get WebSocket URL from backend store
    const { getWsUrl } = useBackendStore.getState();
    const wsUrl = getWsUrl();

    try {
      ws.current = new WebSocket(wsUrl);

      ws.current.onopen = () => {
        console.log('[WebSocket] Connected to virtual company stream');
        setConnected(true);
        reconnectAttempts.current = 0;
      };

      ws.current.onmessage = (event) => {
        try {
          const data: CompanyEvent = JSON.parse(event.data);
          handleEvent(data);
        } catch (error) {
          console.error('[WebSocket] Failed to parse event:', error);
        }
      };

      ws.current.onclose = () => {
        console.log('[WebSocket] Disconnected');
        setConnected(false);
        attemptReconnect();
      };

      ws.current.onerror = (error) => {
        console.error('[WebSocket] Error:', error);
        setConnected(false);
      };
    } catch (error) {
      console.error('[WebSocket] Failed to create connection:', error);
      setConnected(false);
      attemptReconnect();
    }
  }, []);

  const handleEvent = useCallback((event: CompanyEvent) => {
    console.log('[WebSocket] Received event:', event.type);
    
    switch (event.type) {
      case 'message_sent':
        const message: Message = {
          id: event.message_id,
          content: event.content,
          sender: event.sender,
          timestamp: new Date(event.timestamp),
          type: 'text',
          status: 'sent',
        };
        addMessage(event.session_id, message);
        break;
        
      case 'agent_typing':
        setTyping(event.session_id, event.agent_id, true);
        // Auto-clear typing after 3 seconds
        setTimeout(() => {
          setTyping(event.session_id, event.agent_id, false);
        }, 3000);
        break;
        
      case 'group_created':
        // Add new session to list
        updateSession({
          id: event.group_id,
          name: event.name,
          type: 'group',
          participants: event.members,
          unreadCount: 0,
          updatedAt: new Date(),
        });
        break;
        
      case 'agent_online':
      case 'agent_offline':
        // Agent status updates - handled by store
        console.log(`[Company] Agent ${event.agent_id} is ${event.type === 'agent_online' ? 'online' : 'offline'}`);
        break;
        
      case 'system':
        console.log('[Company System]', event.message);
        break;
    }
  }, [addMessage, setTyping, updateSession]);

  const attemptReconnect = useCallback(() => {
    if (reconnectAttempts.current >= maxReconnectAttempts) {
      console.log('[WebSocket] Max reconnection attempts reached');
      return;
    }

    reconnectAttempts.current++;
    const delay = Math.min(1000 * Math.pow(2, reconnectAttempts.current), 30000);
    
    console.log(`[WebSocket] Reconnecting in ${delay}ms (attempt ${reconnectAttempts.current})`);
    
    reconnectTimeout.current = setTimeout(() => {
      connect();
    }, delay);
  }, [connect]);

  useEffect(() => {
    connect();

    return () => {
      if (reconnectTimeout.current) {
        clearTimeout(reconnectTimeout.current);
      }
      if (ws.current) {
        ws.current.close();
      }
    };
  }, [connect]);

  return {
    connected,
  };
}
