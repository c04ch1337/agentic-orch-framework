'use client';

import React, { useEffect, useMemo, useState } from 'react';
import Link from 'next/link';
import { sendExecuteRequest, ExecuteResponseBody, ExecuteMetadata } from '../utils/api';

type ChatRole = 'user' | 'assistant';

interface ChatMessage {
  role: ChatRole;
  content: string;
  timestamp: string;
}

function generateSessionId(): string {
  // Simple UUID v4-style generator suitable for client-side session IDs
  // Not cryptographically secure, but sufficient for correlating chat sessions.
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, (c) => {
    const r = (Math.random() * 16) | 0;
    const v = c === 'x' ? r : (r & 0x3) | 0x8;
    return v.toString(16);
  });
}

export default function OrchestratedChatHarnessPage() {
  const [apiKey, setApiKey] = useState<string>('');
  const [authToken, setAuthToken] = useState<string>('');
  const [messageInput, setMessageInput] = useState<string>('');
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [executionPlan, setExecutionPlan] = useState<unknown | null>(null);
  const [sessionId, setSessionId] = useState<string>('');
  const [isSending, setIsSending] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);
  const [logEntries, setLogEntries] = useState<string[]>([]);
  const [lastResponse, setLastResponse] = useState<ExecuteResponseBody | null>(null);

  // Initialize or restore a stable session id from localStorage
  useEffect(() => {
    if (typeof window === 'undefined') {
      return;
    }

    const key = 'phoenix_session_id';
    const existing = window.localStorage.getItem(key);

    if (existing && typeof existing === 'string') {
      setSessionId(existing);
      return;
    }

    const fresh = generateSessionId();
    setSessionId(fresh);
    try {
      window.localStorage.setItem(key, fresh);
    } catch {
      // Swallow storage errors; session ID will still work in memory.
    }
  }, []);

  const canSend = useMemo(
    () =>
      !!apiKey.trim() &&
      !!authToken.trim() &&
      !!messageInput.trim() &&
      !isSending,
    [apiKey, authToken, messageInput, isSending],
  );

  const formattedExecutionPlan = useMemo(() => {
    if (executionPlan == null) {
      return '';
    }

    if (typeof executionPlan === 'string') {
      return executionPlan;
    }

    try {
      return JSON.stringify(executionPlan, null, 2);
    } catch {
      return String(executionPlan);
    }
  }, [executionPlan]);

  const handleSend = async () => {
    setError(null);

    const trimmedMessage = messageInput.trim();
    if (!trimmedMessage) {
      setError('Message cannot be empty.');
      return;
    }
    if (!apiKey.trim()) {
      setError('Phoenix API key (X-PHOENIX-API-KEY) is required.');
      return;
    }
    if (!authToken.trim()) {
      setError('Bearer auth token is required.');
      return;
    }

    const timestamp = new Date().toISOString();
    const userMessage: ChatMessage = {
      role: 'user',
      content: trimmedMessage,
      timestamp,
    };

    setMessages((prev) => [...prev, userMessage]);
    setIsSending(true);
    setLogEntries((prev) => [
      ...prev,
      `[${new Date().toLocaleTimeString()}] Sending request...`,
    ]);

    let sendSucceeded = false;

    try {
      const metadata: ExecuteMetadata = {
        session_id: sessionId || generateSessionId(),
        source: 'frontend-test-harness-ts',
        response_format: 'agi_response',
      };

      const response = await sendExecuteRequest(
        trimmedMessage,
        apiKey.trim(),
        authToken.trim(),
        metadata,
      );

      setLastResponse(response);
      setExecutionPlan(response.execution_plan ?? null);

      const assistantText =
        typeof response.final_answer === 'string' && response.final_answer.trim().length > 0
          ? response.final_answer
          : '[No final_answer returned from orchestrator]';

      const assistantMessage: ChatMessage = {
        role: 'assistant',
        content: assistantText,
        timestamp: new Date().toISOString(),
      };

      setMessages((prev) => [...prev, assistantMessage]);

      setLogEntries((prev) => [
        ...prev,
        `[${new Date().toLocaleTimeString()}] Request succeeded.`,
      ]);

      sendSucceeded = true;
    } catch (err: unknown) {
      console.error('Error sending execute request:', err);
      const message =
        err instanceof Error
          ? err.message
          : 'Unexpected error while sending execute request';
      setError(message);
      setLogEntries((prev) => [
        ...prev,
        `[${new Date().toLocaleTimeString()}] Request failed: ${message}`,
      ]);
    } finally {
      setIsSending(false);
      if (sendSucceeded) {
        setMessageInput('');
      }
    }
  };

  const lastStatus = useMemo(() => {
    if (isSending) {
      return 'Sending...';
    }
    if (error) {
      return 'Last request failed';
    }
    if (logEntries.length > 0) {
      return 'Last request succeeded';
    }
    return 'Idle';
  }, [isSending, error, logEntries.length]);

  return (
    <div className="min-h-screen bg-gray-50 text-gray-900">
      <main className="max-w-5xl mx-auto py-8 px-4 space-y-8">
        <header className="flex items-center justify-between">
          <div>
            <h1 className="text-2xl font-bold">
              Phoenix Orchestrated Chat Test Harness (TS)
            </h1>
            <p className="text-sm text-gray-600 mt-1">
              Use this page to manually exercise the /api/v1/execute orchestrated chat
              endpoint. Provide your API key and bearer token, then send chat messages.
            </p>
          </div>
          <Link
            href="/dashboard"
            className="text-sm text-blue-600 hover:text-blue-800 underline"
          >
            Go to dashboard
          </Link>
        </header>

        {/* Section 1: Credentials */}
        <section className="bg-white shadow-sm rounded-md p-4 space-y-4 border border-gray-200">
          <h2 className="text-lg font-semibold">1. Credentials</h2>
          <p className="text-sm text-gray-600">
            Both the Phoenix API key (for <code>X-PHOENIX-API-KEY</code>) and a bearer
            token (for <code>Authorization: Bearer {'<token>'}</code>) are required
            before sending messages.
          </p>
          <div className="space-y-3">
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">
                Phoenix API Key (X-PHOENIX-API-KEY)
              </label>
              <input
                type="password"
                className="w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
                placeholder="Enter Phoenix API key"
                value={apiKey}
                onChange={(e) => setApiKey(e.target.value)}
              />
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">
                Bearer Token (Authorization header)
              </label>
              <input
                type="password"
                className="w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
                placeholder="Enter bearer token"
                value={authToken}
                onChange={(e) => setAuthToken(e.target.value)}
              />
            </div>
            <div className="text-xs text-gray-500">
              Session ID: <span className="font-mono">{sessionId || 'initializing...'}</span>
            </div>
          </div>
        </section>

        {/* Section 2: Chat area */}
        <section className="bg-white shadow-sm rounded-md p-4 space-y-4 border border-gray-200">
          <h2 className="text-lg font-semibold">2. Chat</h2>
          <div className="h-64 overflow-y-auto border border-gray-200 rounded-md bg-gray-50 p-3 space-y-3">
            {messages.length === 0 ? (
              <p className="text-sm text-gray-500">
                No messages yet. Enter a prompt below and click "Send" to begin.
              </p>
            ) : (
              messages.map((msg, idx) => (
                <div
                  key={`${msg.timestamp}-${idx}`}
                  className={`p-2 rounded-md text-sm ${msg.role === 'user'
                    ? 'bg-blue-50 border border-blue-100'
                    : 'bg-green-50 border border-green-100'
                    }`}
                >
                  <div className="flex items-center justify-between mb-1">
                    <span className="font-semibold">
                      {msg.role === 'user' ? 'User' : 'Assistant'}
                    </span>
                    <span className="text-xs text-gray-500">
                      {new Date(msg.timestamp).toLocaleTimeString()}
                    </span>
                  </div>
                  <div className="whitespace-pre-wrap break-words">{msg.content}</div>
                </div>
              ))
            )}
          </div>
        </section>

        {/* Section 3: Message composer */}
        <section className="bg-white shadow-sm rounded-md p-4 space-y-3 border border-gray-200">
          <h2 className="text-lg font-semibold">3. Message Composer</h2>
          <textarea
            className="w-full min-h-[80px] rounded-md border border-gray-300 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
            placeholder="Ask a question or provide instructions to the orchestrator..."
            value={messageInput}
            onChange={(e) => setMessageInput(e.target.value)}
          />
          <div className="flex items-center justify-between">
            <p className="text-xs text-gray-500">
              Send is disabled until credentials and a non-empty message are provided.
            </p>
            <button
              type="button"
              className={`inline-flex items-center px-4 py-2 rounded-md text-sm font-medium text-white ${canSend ? 'bg-blue-600 hover:bg-blue-700' : 'bg-gray-400 cursor-not-allowed'
                }`}
              onClick={handleSend}
              disabled={!canSend}
            >
              {isSending ? 'Sending...' : 'Send'}
            </button>
          </div>
        </section>

        {/* Section 4: Execution plan output */}
        <section className="bg-white shadow-sm rounded-md p-4 space-y-3 border border-gray-200">
          <h2 className="text-lg font-semibold">4. Execution Plan (latest response)</h2>
          {executionPlan == null ? (
            <p className="text-sm text-gray-500">No execution_plan received yet.</p>
          ) : (
            <pre className="text-xs bg-gray-900 text-gray-100 rounded-md p-3 overflow-x-auto max-h-64">
              {formattedExecutionPlan}
            </pre>
          )}
        </section>

        {/* Section 5: Logs / status */}
        <section className="bg-white shadow-sm rounded-md p-4 space-y-3 border border-gray-200">
          <h2 className="text-lg font-semibold">5. Status & Logs</h2>
          <div className="text-sm">
            <div>
              <span className="font-medium">Last request status:</span>{' '}
              <span>{lastStatus}</span>
            </div>
            {error && (
              <div className="mt-2 text-sm text-red-700 bg-red-50 border border-red-200 rounded-md p-2">
                <span className="font-semibold">Error:</span> {error}
              </div>
            )}
          </div>
          <div>
            <h3 className="text-sm font-semibold mb-1">Log entries</h3>
            {logEntries.length === 0 ? (
              <p className="text-xs text-gray-500">No log entries yet.</p>
            ) : (
              <ul className="text-xs max-h-40 overflow-y-auto list-disc list-inside space-y-1">
                {logEntries.map((entry, idx) => (
                  <li key={idx}>{entry}</li>
                ))}
              </ul>
            )}
          </div>
          {lastResponse && (
            <details className="mt-3 text-xs">
              <summary className="cursor-pointer text-blue-600">
                Show raw last response payload
              </summary>
              <pre className="mt-2 bg-gray-900 text-gray-100 rounded-md p-3 overflow-x-auto max-h-64">
                {JSON.stringify(lastResponse, null, 2)}
              </pre>
            </details>
          )}
        </section>
      </main>
    </div>
  );
}
