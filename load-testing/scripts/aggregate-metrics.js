/**
 * Phoenix Orchestrator Load Testing Results Aggregator
 * 
 * This script processes k6 output files and aggregates metrics into summary reports
 * with detailed statistical analysis. It can be run after test execution to generate
 * CSV files and summary reports for further analysis.
 */

const fs = require('fs');
const path = require('path');
const readline = require('readline');

// Configuration
const DEFAULT_RESULTS_DIR = '../results';
const DEFAULT_OUTPUT_DIR = '../results/aggregated';
const PERCENTILES = [0.5, 0.75, 0.9, 0.95, 0.99];

// Command line arguments
const args = process.argv.slice(2);
const resultsDir = args[0] || DEFAULT_RESULTS_DIR;
const outputDir = args[1] || DEFAULT_OUTPUT_DIR;

// Ensure output directory exists
if (!fs.existsSync(outputDir)) {
    fs.mkdirSync(outputDir, { recursive: true });
}

// Log function
function log(message) {
    console.log(`[${new Date().toISOString()}] ${message}`);
}

// Helper function to calculate percentiles
function percentile(arr, p) {
    if (arr.length === 0) return 0;
    const sorted = [...arr].sort((a, b) => a - b);
    const index = Math.floor(sorted.length * p);
    return sorted[index];
}

// Helper function to calculate statistics
function calculateStats(values) {
    if (values.length === 0) return {
        min: 0,
        max: 0,
        avg: 0,
        median: 0,
        p75: 0,
        p90: 0,
        p95: 0,
        p99: 0,
        count: 0,
        sum: 0,
        stdDev: 0
    };

    const sum = values.reduce((a, b) => a + b, 0);
    const avg = sum / values.length;
    const variance = values.reduce((a, b) => a + Math.pow(b - avg, 2), 0) / values.length;
    const stdDev = Math.sqrt(variance);

    return {
        min: Math.min(...values),
        max: Math.max(...values),
        avg,
        median: percentile(values, 0.5),
        p75: percentile(values, 0.75),
        p90: percentile(values, 0.9),
        p95: percentile(values, 0.95),
        p99: percentile(values, 0.99),
        count: values.length,
        sum,
        stdDev
    };
}

// Process a single k6 result file
async function processResultFile(filePath) {
    log(`Processing file: ${filePath}`);

    // Initialize metrics
    const metrics = {
        http_reqs: [],
        http_req_duration: [],
        http_req_failed: [],
        iterations: [],
        vus: [],
        custom: {}
    };

    // Read the file
    const fileStream = fs.createReadStream(filePath);
    const rl = readline.createInterface({
        input: fileStream,
        crlfDelay: Infinity
    });

    // Track points in time
    const timePoints = [];
    const timePointValues = {};

    // Track test metadata
    let testConfig = {};
    let testInfo = {
        startTime: null,
        endTime: null,
        scenarioName: path.basename(filePath, '.json')
    };

    // Parse each line of the file
    for await (const line of rl) {
        try {
            const data = JSON.parse(line);

            // Extract test config
            if (data.type === 'init') {
                testConfig = data.data;
                testInfo.startTime = new Date(data.timestamp);
            }

            // Extract metrics
            if (data.type === 'metric' && data.data) {
                const { name, value, time } = data.data;
                const metricTime = new Date(time);

                // Record test end time
                if (!testInfo.endTime || metricTime > testInfo.endTime) {
                    testInfo.endTime = metricTime;
                }

                // Add time point if not exists
                if (!timePoints.includes(time)) {
                    timePoints.push(time);
                    timePointValues[time] = {};
                }

                // Store value at time point
                timePointValues[time][name] = value;

                // Add to appropriate metric collection
                if (metrics[name] !== undefined) {
                    metrics[name].push(value);
                } else {
                    // Handle custom metrics
                    if (!metrics.custom[name]) {
                        metrics.custom[name] = [];
                    }
                    metrics.custom[name].push(value);
                }
            }
        } catch (error) {
            log(`Error processing line in ${filePath}: ${error.message}`);
        }
    }

    // Calculate summary statistics
    const summary = {
        testInfo,
        testConfig,
        metrics: {
            http_reqs: calculateStats(metrics.http_reqs),
            http_req_duration: calculateStats(metrics.http_req_duration),
            http_req_failed: {
                rate: metrics.http_req_failed.filter(v => v === true).length / metrics.http_req_failed.length,
                count: metrics.http_req_failed.length,
                failures: metrics.http_req_failed.filter(v => v === true).length
            },
            iterations: calculateStats(metrics.iterations),
            vus: calculateStats(metrics.vus),
            custom: {}
        },
        timePoints: timePoints.sort(),
        timePointValues
    };

    // Calculate stats for custom metrics
    Object.keys(metrics.custom).forEach(name => {
        summary.metrics.custom[name] = calculateStats(metrics.custom[name]);
    });

    return summary;
}

// Generate CSV report
function generateCSV(summary, outputPath) {
    log(`Generating CSV report: ${outputPath}`);

    const headers = [
        'Timestamp',
        'VUs',
        'HTTP Requests',
        'HTTP Request Duration (ms)',
        'HTTP Request Failures',
        'Iterations'
    ];

    // Add custom metrics
    const customMetrics = Object.keys(summary.metrics.custom);
    headers.push(...customMetrics);

    // Create CSV content
    let csvContent = headers.join(',') + '\n';

    // Add data for each time point
    summary.timePoints.forEach(time => {
        const values = summary.timePointValues[time];
        const row = [
            time,
            values.vus || '',
            values.http_reqs || '',
            values.http_req_duration || '',
            values.http_req_failed || '',
            values.iterations || ''
        ];

        // Add custom metrics values
        customMetrics.forEach(metric => {
            row.push(values[metric] || '');
        });

        csvContent += row.join(',') + '\n';
    });

    fs.writeFileSync(outputPath, csvContent);
}

// Generate summary report
function generateSummaryReport(summaries, outputPath) {
    log(`Generating summary report: ${outputPath}`);

    const report = {
        generatedAt: new Date().toISOString(),
        testCount: summaries.length,
        tests: summaries.map(s => ({
            name: s.testInfo.scenarioName,
            startTime: s.testInfo.startTime,
            endTime: s.testInfo.endTime,
            duration: s.testInfo.endTime - s.testInfo.startTime,
            requestCount: s.metrics.http_reqs.count,
            errorRate: s.metrics.http_req_failed.rate,
            avgResponseTime: s.metrics.http_req_duration.avg,
            p95ResponseTime: s.metrics.http_req_duration.p95,
            maxResponseTime: s.metrics.http_req_duration.max,
            avgVUs: s.metrics.vus.avg,
            maxVUs: s.metrics.vus.max,
        })),
        aggregatedMetrics: {
            totalRequests: summaries.reduce((sum, s) => sum + s.metrics.http_reqs.count, 0),
            totalIterations: summaries.reduce((sum, s) => sum + s.metrics.iterations.count, 0),
            avgErrorRate: summaries.reduce((sum, s) => sum + s.metrics.http_req_failed.rate, 0) / summaries.length,
            avgResponseTime: summaries.reduce((sum, s) => sum + s.metrics.http_req_duration.avg, 0) / summaries.length,
            p95ResponseTime: summaries.reduce((sum, s) => sum + s.metrics.http_req_duration.p95, 0) / summaries.length,
            maxResponseTime: Math.max(...summaries.map(s => s.metrics.http_req_duration.max)),
        }
    };

    fs.writeFileSync(outputPath, JSON.stringify(report, null, 2));
}

// Generate prometheus-compatible format for importable metrics
function generatePrometheusMetrics(summary, outputPath) {
    log(`Generating Prometheus metrics: ${outputPath}`);

    let content = '';

    // Add HTTP request metrics
    content += `# HELP phoenix_loadtest_http_requests Total number of HTTP requests\n`;
    content += `# TYPE phoenix_loadtest_http_requests counter\n`;
    content += `phoenix_loadtest_http_requests{test="${summary.testInfo.scenarioName}"} ${summary.metrics.http_reqs.count}\n\n`;

    // Add HTTP duration metrics
    content += `# HELP phoenix_loadtest_http_req_duration HTTP request duration in ms\n`;
    content += `# TYPE phoenix_loadtest_http_req_duration gauge\n`;
    content += `phoenix_loadtest_http_req_duration{test="${summary.testInfo.scenarioName}",quantile="0.5"} ${summary.metrics.http_req_duration.median}\n`;
    content += `phoenix_loadtest_http_req_duration{test="${summary.testInfo.scenarioName}",quantile="0.75"} ${summary.metrics.http_req_duration.p75}\n`;
    content += `phoenix_loadtest_http_req_duration{test="${summary.testInfo.scenarioName}",quantile="0.9"} ${summary.metrics.http_req_duration.p90}\n`;
    content += `phoenix_loadtest_http_req_duration{test="${summary.testInfo.scenarioName}",quantile="0.95"} ${summary.metrics.http_req_duration.p95}\n`;
    content += `phoenix_loadtest_http_req_duration{test="${summary.testInfo.scenarioName}",quantile="0.99"} ${summary.metrics.http_req_duration.p99}\n\n`;

    // Add HTTP failures
    content += `# HELP phoenix_loadtest_http_failures HTTP request failures\n`;
    content += `# TYPE phoenix_loadtest_http_failures gauge\n`;
    content += `phoenix_loadtest_http_failures{test="${summary.testInfo.scenarioName}"} ${summary.metrics.http_req_failed.failures}\n\n`;

    // Add custom metrics
    Object.keys(summary.metrics.custom).forEach(metricName => {
        const metric = summary.metrics.custom[metricName];
        const prometheusName = metricName.replace(/[^a-zA-Z0-9_]/g, '_');

        content += `# HELP phoenix_loadtest_${prometheusName} Custom metric: ${metricName}\n`;
        content += `# TYPE phoenix_loadtest_${prometheusName} gauge\n`;
        content += `phoenix_loadtest_${prometheusName}{test="${summary.testInfo.scenarioName}",stat="avg"} ${metric.avg}\n`;
        content += `phoenix_loadtest_${prometheusName}{test="${summary.testInfo.scenarioName}",stat="p95"} ${metric.p95}\n`;
        content += `phoenix_loadtest_${prometheusName}{test="${summary.testInfo.scenarioName}",stat="max"} ${metric.max}\n\n`;
    });

    fs.writeFileSync(outputPath, content);
}

// Main function to process all result files
async function processAllResults() {
    log(`Starting metrics aggregation from ${resultsDir} to ${outputDir}`);

    // Get all JSON files in the results directory
    const files = fs.readdirSync(resultsDir)
        .filter(file => file.endsWith('.json'))
        .map(file => path.join(resultsDir, file));

    if (files.length === 0) {
        log('No result files found!');
        return;
    }

    log(`Found ${files.length} result files to process`);

    // Process each file
    const summaries = [];
    for (const file of files) {
        try {
            const summary = await processResultFile(file);
            summaries.push(summary);

            // Generate individual file outputs
            const baseName = path.basename(file, '.json');
            generateCSV(summary, path.join(outputDir, `${baseName}.csv`));
            generatePrometheusMetrics(summary, path.join(outputDir, `${baseName}.prom`));
        } catch (error) {
            log(`Error processing ${file}: ${error.message}`);
        }
    }

    // Generate aggregated reports
    if (summaries.length > 0) {
        generateSummaryReport(summaries, path.join(outputDir, 'summary-report.json'));

        // Generate combined CSV for all results
        const combinedCSV = path.join(outputDir, 'all-results.csv');
        const headers = [
            'Test',
            'Start Time',
            'End Time',
            'Duration (ms)',
            'Request Count',
            'Error Rate',
            'Avg Response Time (ms)',
            'p95 Response Time (ms)',
            'Max Response Time (ms)'
        ];

        let csvContent = headers.join(',') + '\n';

        summaries.forEach(summary => {
            const row = [
                summary.testInfo.scenarioName,
                summary.testInfo.startTime.toISOString(),
                summary.testInfo.endTime.toISOString(),
                summary.testInfo.endTime - summary.testInfo.startTime,
                summary.metrics.http_reqs.count,
                summary.metrics.http_req_failed.rate,
                summary.metrics.http_req_duration.avg,
                summary.metrics.http_req_duration.p95,
                summary.metrics.http_req_duration.max
            ];

            csvContent += row.join(',') + '\n';
        });

        fs.writeFileSync(combinedCSV, csvContent);
        log(`Generated combined CSV report: ${combinedCSV}`);
    }

    log('Metrics aggregation completed successfully');
}

// Run the main function
processAllResults().catch(error => {
    console.error('Fatal error:', error);
    process.exit(1);
});