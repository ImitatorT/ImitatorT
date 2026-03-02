import { useEffect, useRef, useCallback, useState } from 'react';
import { useChatStore } from '../stores/appStore';
import { useBackendStore } from '../stores/backendStore';
import type { CompanyEvent, Message } from '../types';

export function useWebSocket() {
  const ws = useRef<WebSocket | null>(null);
  const reconnectTimeout = useRef<NodeJS.Timeout | null>(null);
  const reconnectAttempts = useRef(0);
  const maxReconnectAttempts = 5;

  const [connected, setConnected] = useState(false);
  const { addMessage, setTyping, updateSession } = useChatStore();

  // Store functions in refs to avoid circular dependencies
  const connectRef = useRef<() => void>();
  const attemptReconnectRef = useRef<() => void>();

  const attemptReconnect = useCallback(() => {
    if (reconnectAttempts.current >= maxReconnectAttempts) {
      return;
    }

    reconnectAttempts.current++;
    const delay = Math.min(1000 * Math.pow(2, reconnectAttempts.current), 30000);

    if (reconnectTimeout.current) {
      clearTimeout(reconnectTimeout.current);
    }
    reconnectTimeout.current = setTimeout(() => {
      connectRef.current?.();
    }, delay);
  }, []);

  const connect = useCallback(() => {
    if (ws.current?.readyState === WebSocket.OPEN) return;

    const { getWsUrl } = useBackendStore.getState();
    const wsUrl = getWsUrl();

    try {
      ws.current = new WebSocket(wsUrl);

      ws.current.onopen = () => {
        setConnected(true);
        reconnectAttempts.current = 0;
      };

      ws.current.onmessage = (event) => {
        try {
          const data: CompanyEvent = JSON.parse(event.data);
          switch (data.type) {
            case 'message_sent': {
              const message: Message = {
                id: data.message_id,
                content: data.content,
                sender: data.sender,
                timestamp: new Date(data.timestamp),
                type: 'text',
                status: 'sent',
              };
              addMessage(data.session_id, message);
              break;
            }

            case 'agent_typing':
              setTyping(data.session_id, data.agent_id, true);
              setTimeout(() => {
                setTyping(data.session_id, data.agent_id, false);
              }, 3000);
              break;

            case 'group_created':
              updateSession({
                id: data.group_id,
                name: data.name,
                type: 'group',
                participants: data.members,
                unreadCount: 0,
                updatedAt: new Date(),
              });
              break;

            case 'agent_online':
            case 'agent_offline':
              break;

            case 'system':
              break;
          }
        } catch (_error) {
          // Handle parsing error
        }
      };

      ws.current.onclose = () => {
        setConnected(false);
        attemptReconnectRef.current?.();
      };

      ws.current.onerror = (_error) => {
        setConnected(false);
      };
    } catch (_error) {
      setConnected(false);
      attemptReconnectRef.current?.();
    }
  }, [addMessage, setTyping, updateSession]);

  // Update refs whenever the functions change
  useEffect(() => {
    connectRef.current = connect;
    attemptReconnectRef.current = attemptReconnect;
  }, [connect, attemptReconnect]);

  useEffect(() => {
    connect();

    return () => {
      if (reconnectTimeout.current) {
        clearTimeout(reconnectTimeout.current);
        reconnectTimeout.current = null;
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