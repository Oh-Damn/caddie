import type { ClientMessage, ServerMessage } from '@companion/protocol';
import { PROTOCOL_VERSION } from '@companion/protocol';
import { newId } from './session';

export function envelope<T extends ClientMessage['type']>(
  type: T,
  payload: Extract<ClientMessage, { type: T }>['payload'],
): ClientMessage {
  return {
    v: PROTOCOL_VERSION,
    id: newId(),
    type,
    payload,
  } as ClientMessage;
}

export function parseServer(raw: string): ServerMessage | null {
  try {
    const msg = JSON.parse(raw) as ServerMessage;
    if (msg.v !== PROTOCOL_VERSION || typeof msg.type !== 'string') return null;
    return msg;
  } catch {
    return null;
  }
}
