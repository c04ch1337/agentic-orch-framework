/**
 * API Key Storage Utilities
 * 
 * Functions for securely storing and retrieving the API key
 */

const API_KEY_STORAGE_KEY = 'phoenix_orch_api_key';

/**
 * Save API key to browser storage
 */
export function saveApiKey(apiKey: string): void {
    try {
        localStorage.setItem(API_KEY_STORAGE_KEY, apiKey);
    } catch (error) {
        console.error('Failed to save API key:', error);
    }
}

/**
 * Get API key from browser storage
 */
export function getApiKey(): string | null {
    try {
        return localStorage.getItem(API_KEY_STORAGE_KEY);
    } catch (error) {
        console.error('Failed to retrieve API key:', error);
        return null;
    }
}

/**
 * Clear API key from browser storage
 */
export function clearApiKey(): void {
    try {
        localStorage.removeItem(API_KEY_STORAGE_KEY);
    } catch (error) {
        console.error('Failed to clear API key:', error);
    }
}

/**
 * Check if API key exists in storage
 */
export function hasApiKey(): boolean {
    return !!getApiKey();
}

/**
 * Validate API key format
 */
export function isValidApiKeyFormat(apiKey: string): boolean {
    // Simple validation - adjust based on actual API key format requirements
    return apiKey.length >= 8;
}