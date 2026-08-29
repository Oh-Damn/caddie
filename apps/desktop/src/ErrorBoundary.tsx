import { invoke } from '@tauri-apps/api/core';
import { Component, type ReactNode } from 'react';

type Props = {
  children: ReactNode;
};

type State = {
  error: string | null;
};

export class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { error: null };
  }

  static getDerivedStateFromError(error: Error): State {
    return { error: error.stack ?? error.message };
  }

  componentDidCatch(error: Error) {
    void invoke('dev_client_error', { message: error.stack ?? error.message });
  }

  render() {
    if (this.state.error) {
      return (
        <div className="min-h-screen bg-app px-6 py-8 text-danger">
          <p className="font-medium">UI crashed</p>
          <p className="mt-2 whitespace-pre-wrap text-sm">{this.state.error}</p>
        </div>
      );
    }
    return this.props.children;
  }
}
