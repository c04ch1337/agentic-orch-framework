/**
 * Type definitions for the Phoenix ORCH Dashboard
 */

// API Response Types
export interface ApiResponse<T> {
    success: boolean;
    data?: T;
    error?: string;
}

// Message structure for chat
export interface Message {
    id: string;
    content: string;
    isUser: boolean;
    timestamp: Date;
    response?: ExecuteResponse;
}

// Execute Response structure
export interface ExecuteResponse {
    result: string;
    metadata: {
        plan: string[];
        routedTo: string;
        payload: Record<string, any>;
        thoughtProcess: string;
    };
}

// User settings
export interface UserSettings {
    apiKey: string;
}

// Theme settings
export interface Theme {
    darkMode: boolean;
}

// API connection status
export interface ApiStatus {
    connected: boolean;
    lastChecked: Date | null;
}