import { useEffect } from 'react';
import { haptic } from '../../companion/session';
import { Button } from './Button';

type Props = {
  message: string;
  onDismiss: () => void;
};

export function ErrorPopup({ message, onDismiss }: Props) {
  useEffect(() => {
    haptic('warning');
  }, [message]);

  return (
    <div
      className="fixed inset-0 z-30 flex items-center justify-center bg-app-well/80 px-4"
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="error-title"
      aria-describedby="error-message"
    >
      <div className="faceplate w-full max-w-sm p-4">
        <h2 id="error-title" className="type-micro text-danger">
          Error
        </h2>
        <p id="error-message" className="type-secondary mt-2">
          {message}
        </p>
        <div className="mt-4 flex flex-col">
          <Button onClick={onDismiss}>OK</Button>
        </div>
      </div>
    </div>
  );
}
