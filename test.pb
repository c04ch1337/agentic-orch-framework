
µÊ
.proto/agi_core.protoagi_core"ﬂ
Request
id (	Rid
service (	Rservice
method (	Rmethod
payload (Rpayload;
metadata (2.agi_core.Request.MetadataEntryRmetadata;
MetadataEntry
key (	Rkey
value (	Rvalue:8"Ê
Response
id (	Rid
status_code (R
statusCode
payload (Rpayload
error (	Rerror<
metadata (2 .agi_core.Response.MetadataEntryRmetadata;
MetadataEntry
key (	Rkey
value (	Rvalue:8"ﬁ
AgiResponse!
final_answer (	RfinalAnswer%
execution_plan (	RexecutionPlan%
routed_service (	RroutedService,
phoenix_session_id (	RphoenixSessionId0
output_artifact_urls (	RoutputArtifactUrls"b
RouteRequest%
target_service (	RtargetService+
request (2.agi_core.RequestRrequest"\
RouteResponse.
response (2.agi_core.ResponseRresponse
	routed_to (	RroutedTo"1
ServiceQuery!
service_name (	RserviceName"?
ServiceEndpoint
address (	Raddress
port (Rport"≥
GenerateRequest
prompt (	RpromptI

parameters (2).agi_core.GenerateRequest.ParametersEntryR
parameters=
ParametersEntry
key (	Rkey
value (	Rvalue:8"©
GenerateResponse
text (	RtextD
metadata (2(.agi_core.GenerateResponse.MetadataEntryRmetadata;
MetadataEntry
key (	Rkey
value (	Rvalue:8"E
LLMProcessRequest
text (	Rtext
	operation (	R	operation"±
LLMProcessResponse
result (	RresultF
metadata (2*.agi_core.LLMProcessResponse.MetadataEntryRmetadata;
MetadataEntry
key (	Rkey
value (	Rvalue:8"∞
ToolRequest
	tool_name (	RtoolNameE

parameters (2%.agi_core.ToolRequest.ParametersEntryR
parameters=
ParametersEntry
key (	Rkey
value (	Rvalue:8"V
ToolResponse
success (Rsuccess
result (	Rresult
error (	Rerror".
ListToolsRequest
category (	Rcategory")
ListToolsResponse
tools (	Rtools"¿
ValidationRequest+
request (2.agi_core.RequestRrequestB
context (2(.agi_core.ValidationRequest.ContextEntryRcontext:
ContextEntry
key (	Rkey
value (	Rvalue:8"g
ValidationResponse
approved (Rapproved
reason (	Rreason

risk_level (R	riskLevel"?
ThreatCheck
content (	Rcontent
source (	Rsource"n
ThreatResponse
	is_threat (RisThreat
threat_type (	R
threatType

confidence (R
confidence"Ì
LogEntry
level (	Rlevel
message (	Rmessage
service (	Rservice<
metadata (2 .agi_core.LogEntry.MetadataEntryRmetadata
	timestamp (R	timestamp;
MetadataEntry
key (	Rkey
value (	Rvalue:8">
LogResponse
success (Rsuccess
log_id (	RlogId"Ö
MetricsRequest
service (	Rservice
metric_name (	R
metricName

start_time (R	startTime
end_time (RendTime"è
MetricsResponse@
metrics (2&.agi_core.MetricsResponse.MetricsEntryRmetrics:
MetricsEntry
key (	Rkey
value (Rvalue:8"µ
StoreRequest
key (	Rkey
value (Rvalue@
metadata (2$.agi_core.StoreRequest.MetadataEntryRmetadata;
MetadataEntry
key (	Rkey
value (	Rvalue:8"F
StoreResponse
success (Rsuccess
	stored_id (	RstoredId"°
RetrieveRequest
key (	Rkey@
filters (2&.agi_core.RetrieveRequest.FiltersEntryRfilters:
FiltersEntry
key (	Rkey
value (	Rvalue:8"¡
RetrieveResponse
value (RvalueD
metadata (2(.agi_core.RetrieveResponse.MetadataEntryRmetadata
found (Rfound;
MetadataEntry
key (	Rkey
value (	Rvalue:8"¡
QueryRequest
query (	RqueryF

parameters (2&.agi_core.QueryRequest.ParametersEntryR
parameters
limit (Rlimit=
ParametersEntry
key (	Rkey
value (	Rvalue:8"ø
QueryResponse
results (Rresults
count (RcountA
metadata (2%.agi_core.QueryResponse.MetadataEntryRmetadata;
MetadataEntry
key (	Rkey
value (	Rvalue:8"´
CommandRequest
command (	Rcommand
args (	Rargs3
env (2!.agi_core.CommandRequest.EnvEntryRenv6
EnvEntry
key (	Rkey
value (	Rvalue:8"^
CommandResponse
stdout (	Rstdout
stderr (	Rstderr
	exit_code (RexitCode"¥
InputRequest

input_type (	R	inputTypeF

parameters (2&.agi_core.InputRequest.ParametersEntryR
parameters=
ParametersEntry
key (	Rkey
value (	Rvalue:8"?
InputResponse
success (Rsuccess
error (	Rerror"2
HealthRequest!
service_name (	RserviceName"ù
HealthResponse
healthy (Rhealthy!
service_name (	RserviceName%
uptime_seconds (RuptimeSeconds
status (	RstatusN
dependencies (2*.agi_core.HealthResponse.DependenciesEntryRdependencies?
DependenciesEntry
key (	Rkey
value (	Rvalue:8"±
ContextRequest

request_id (	R	requestId
query (	Rquery

agent_type (	R	agentType,
max_context_tokens (RmaxContextTokens

kb_sources (	R	kbSources"Î
EnrichedContext

request_id (	R	requestId%
original_query (	RoriginalQuery#
system_prompt (	RsystemPrompt?
context_entries (2.agi_core.ContextEntryRcontextEntries*
total_tokens_used (RtotalTokensUsedC
metadata (2'.agi_core.EnrichedContext.MetadataEntryRmetadata;
MetadataEntry
key (	Rkey
value (	Rvalue:8"å
ContextEntry
	source_kb (	RsourceKb
content (	Rcontent'
relevance_score (RrelevanceScore
	timestamp (R	timestamp"Y
ContextQuery
query (	Rquery
limit (Rlimit

kb_sources (	R	kbSources"d
ContextResponse0
entries (2.agi_core.ContextEntryRentries
total_count (R
totalCount"è
ContextSummarySchema
	schema_id (	RschemaId+
field_definitions (	RfieldDefinitions-
schema_description (	RschemaDescription"Ú
RawContextData
user_id (	RuserId0
entries (2.agi_core.ContextEntryRentries
query (	RqueryB
metadata (2&.agi_core.RawContextData.MetadataEntryRmetadata;
MetadataEntry
key (	Rkey
value (	Rvalue:8"œ
CompileContextRequest

request_id (	R	requestId3
raw_data (2.agi_core.RawContextDataRrawData6
schema (2.agi_core.ContextSummarySchemaRschema*
max_output_tokens (RmaxOutputTokens"à
CompiledContextResponse

request_id (	R	requestId#
compiled_json (	RcompiledJson
tokens_used (R
tokensUsedK
metadata (2/.agi_core.CompiledContextResponse.MetadataEntryRmetadata;
MetadataEntry
key (	Rkey
value (	Rvalue:8"ï
ReflectionRequest

request_id (	R	requestId-
action_description (	RactionDescription
outcome (	Routcome
success (RsuccessB
context (2(.agi_core.ReflectionRequest.ContextEntryRcontext:
ContextEntry
key (	Rkey
value (	Rvalue:8"»
ReflectionResult

request_id (	R	requestId
analysis (	Ranalysis'
lessons_learned (	RlessonsLearned"
improvements (	Rimprovements)
confidence_score (RconfidenceScoreD
metadata (2(.agi_core.ReflectionResult.MetadataEntryRmetadata;
MetadataEntry
key (	Rkey
value (	Rvalue:8"ë
EvaluationRequest

request_id (	R	requestId'
proposed_action (	RproposedAction
goal (	Rgoal 
constraints (	RconstraintsB
context (2(.agi_core.EvaluationRequest.ContextEntryRcontext:
ContextEntry
key (	Rkey
value (	Rvalue:8"Ÿ
EvaluationResult

request_id (	R	requestId 
recommended (Rrecommended
	rationale (	R	rationale
risks (	Rrisks"
alternatives (	Ralternatives)
confidence_score (RconfidenceScoreD
metadata (2(.agi_core.EvaluationResult.MetadataEntryRmetadata;
MetadataEntry
key (	Rkey
value (	Rvalue:8"a
MetaCognitiveRequest

request_id (	R	requestId
topic (	Rtopic
depth (Rdepth"ƒ
MetaCognitiveResult

request_id (	R	requestId'
self_assessment (	RselfAssessment
	strengths (	R	strengths

weaknesses (	R
weaknesses!
growth_areas (	RgrowthAreasG
metadata (2+.agi_core.MetaCognitiveResult.MetadataEntryRmetadata;
MetadataEntry
key (	Rkey
value (	Rvalue:8"î
ScheduleTaskRequest
task_id (	RtaskId
	task_name (	RtaskName'
cron_expression (	RcronExpression
payload (	RpayloadG
metadata (2+.agi_core.ScheduleTaskRequest.MetadataEntryRmetadata;
MetadataEntry
key (	Rkey
value (	Rvalue:8"ç
ScheduleTaskResponse
success (Rsuccess!
scheduled_id (	RscheduledId"
next_run_time (	RnextRunTime
error (	Rerror"@
ListTasksRequest
filter (	Rfilter
limit (Rlimit"Î
ScheduledTask
task_id (	RtaskId
	task_name (	RtaskName'
cron_expression (	RcronExpression
status (	Rstatus"
next_run_time (	RnextRunTime"
last_run_time (	RlastRunTime
	run_count (RrunCount"c
ListTasksResponse-
tasks (2.agi_core.ScheduledTaskRtasks
total_count (R
totalCount",
CancelTaskRequest
task_id (	RtaskId"D
CancelTaskResponse
success (Rsuccess
error (	Rerror"˝
RegisterAgentRequest
name (	Rname
port (Rport
role (	Rrole"
capabilities (	RcapabilitiesH
metadata (2,.agi_core.RegisterAgentRequest.MetadataEntryRmetadata;
MetadataEntry
key (	Rkey
value (	Rvalue:8"b
RegisterAgentResponse
success (Rsuccess
agent_id (	RagentId
error (	Rerror"E
GetAgentRequest
name (	Rname

capability (	R
capability"ö
	AgentInfo
agent_id (	RagentId
name (	Rname
port (Rport
role (	Rrole"
capabilities (	Rcapabilities
status (	Rstatus=
metadata (2!.agi_core.AgentInfo.MetadataEntryRmetadata;
MetadataEntry
key (	Rkey
value (	Rvalue:8"S
GetAgentResponse
found (Rfound)
agent (2.agi_core.AgentInfoRagent"e
ListAgentsRequest+
capability_filter (	RcapabilityFilter#
status_filter (	RstatusFilter"b
ListAgentsResponse+
agents (2.agi_core.AgentInfoRagents
total_count (R
totalCount"!
GetAvailableCapabilitiesRequest"Ü
CapabilityMetadata 
description (	Rdescription'
required_params (	RrequiredParams%
provider_agent (	RproviderAgent"˜
 GetAvailableCapabilitiesResponse"
capabilities (	RcapabilitiesT
metadata (28.agi_core.GetAvailableCapabilitiesResponse.MetadataEntryRmetadataY
MetadataEntry
key (	Rkey2
value (2.agi_core.CapabilityMetadataRvalue:8"»
ScanRequest
target (	Rtarget
	scan_type (	RscanTypeE

parameters (2%.agi_core.ScanRequest.ParametersEntryR
parameters=
ParametersEntry
key (	Rkey
value (	Rvalue:8"û

ScanResult
scan_id (	RscanIdA
vulnerabilities (2.agi_core.VulnerabilityRvulnerabilities
summary (	Rsummary

risk_score (R	riskScore>
metadata (2".agi_core.ScanResult.MetadataEntryRmetadata;
MetadataEntry
key (	Rkey
value (	Rvalue:8"ì
Vulnerability
id (	Rid
name (	Rname
severity (	Rseverity 
description (	Rdescription 
remediation (	Rremediation"˝
AttackSimulationRequest
target (	Rtarget
attack_type (	R
attackType
dry_run (RdryRunQ

parameters (21.agi_core.AttackSimulationRequest.ParametersEntryR
parameters=
ParametersEntry
key (	Rkey
value (	Rvalue:8"ÿ
AttackSimulationResult#
simulation_id (	RsimulationId
success (Rsuccess
attack_path (	R
attackPath+
impact_assessment (	RimpactAssessment(
recommendations (	RrecommendationsJ
metadata (2..agi_core.AttackSimulationResult.MetadataEntryRmetadata;
MetadataEntry
key (	Rkey
value (	Rvalue:8"O
ReportRequest
report_type (	R
reportType

time_range (	R	timeRange"∫
SecurityReport
	report_id (	RreportId
report_type (	R
reportType
content (	Rcontent!
key_findings (	RkeyFindings,
overall_risk_score (RoverallRiskScoreB
metadata (2&.agi_core.SecurityReport.MetadataEntryRmetadata;
MetadataEntry
key (	Rkey
value (	Rvalue:8"à
AnomalyTriageRequest

anomaly_id (	R	anomalyId!
anomaly_type (	RanomalyType
data (	Rdata
priority (Rpriority"…
TriageResult
	triage_id (	RtriageId
	is_threat (RisThreat3
threat_classification (	RthreatClassification
severity (Rseverity/
recommended_actions (	RrecommendedActions@
metadata (2$.agi_core.TriageResult.MetadataEntryRmetadata;
MetadataEntry
key (	Rkey
value (	Rvalue:8"õ
ContainmentRequest
	threat_id (	RthreatId)
containment_type (	RcontainmentType
target (	Rtarget%
auto_remediate (RautoRemediate"ì
ContainmentResult%
containment_id (	RcontainmentId
success (Rsuccess!
action_taken (	RactionTaken
status (	RstatusE
metadata (2).agi_core.ContainmentResult.MetadataEntryRmetadata;
MetadataEntry
key (	Rkey
value (	Rvalue:8"|
HardeningRequest
target (	Rtarget+
hardening_profile (	RhardeningProfile#
apply_changes (RapplyChanges"ª
HardeningResult!
hardening_id (	RhardeningId'
changes_applied (	RchangesApplied/
changes_recommended (	RchangesRecommended)
compliance_score (RcomplianceScoreC
metadata (2'.agi_core.HardeningResult.MetadataEntryRmetadata;
MetadataEntry
key (	Rkey
value (	Rvalue:8"√
SentimentFact
	source_id (	RsourceId
	timestamp (R	timestamp1
	sentiment (2.agi_core.SentimentR	sentiment)
confidence_score (RconfidenceScore
raw_text (	RrawTextA
metadata (2%.agi_core.SentimentFact.MetadataEntryRmetadata;
MetadataEntry
key (	Rkey
value (	Rvalue:8"D
StoreSentimentRequest+
fact (2.agi_core.SentimentFactRfact"Ú
StoreSentimentResponse
success (Rsuccess8
sentiment_shift_detected (RsentimentShiftDetectedB
previous_sentiment (2.agi_core.SentimentRpreviousSentiment@
current_sentiment (2.agi_core.SentimentRcurrentSentiment"±
AGIState@
current_sentiment (2.agi_core.SentimentRcurrentSentiment

confidence (R
confidence!
last_updated (RlastUpdated)
dominant_emotion (	RdominantEmotion9
context (2.agi_core.AGIState.ContextEntryRcontext:
ContextEntry
key (	Rkey
value (	Rvalue:8".
GetStateRequest
	source_id (	RsourceId"Ã
UserIdentity
user_id (	RuserId
name (	Rname&
role (2.agi_core.UserRoleRrole 
permissions (	Rpermissions

created_at (R	createdAt
last_active (R
lastActiveF

attributes (2&.agi_core.UserIdentity.AttributesEntryR
attributes=
AttributesEntry
key (	Rkey
value (	Rvalue:8"Ò
CommunicationPreference
user_id (	RuserId-
preferred_language (	RpreferredLanguage#
alert_channel (	RalertChannel+
verbose_responses (RverboseResponses2
response_detail_level (RresponseDetailLevelK
settings (2/.agi_core.CommunicationPreference.SettingsEntryRsettings;
SettingsEntry
key (	Rkey
value (	Rvalue:8"é
RegisterUserRequest2
identity (2.agi_core.UserIdentityRidentityC
preferences (2!.agi_core.CommunicationPreferenceRpreferences"I
RegisterUserResponse
success (Rsuccess
user_id (	RuserId")
GetUserRequest
user_id (	RuserId"†
GetUserResponse
found (Rfound2
identity (2.agi_core.UserIdentityRidentityC
preferences (2!.agi_core.CommunicationPreferenceRpreferences"G
ListUsersRequest3
role_filter (2.agi_core.UserRoleR
roleFilter"b
ListUsersResponse,
users (2.agi_core.UserIdentityRusers
total_count (R
totalCount" 
	CoreValue
value_id (	RvalueId
name (	Rname 
description (	Rdescription3
priority (2.agi_core.ValuePriorityRpriority

constraint (	R
constraint
	is_active (RisActive=
metadata (2!.agi_core.CoreValue.MetadataEntryRmetadata;
MetadataEntry
key (	Rkey
value (	Rvalue:8">
StoreValueRequest)
value (2.agi_core.CoreValueRvalue"I
StoreValueResponse
success (Rsuccess
value_id (	RvalueId"@
GetValueRequest
value_id (	RvalueId
name (	Rname"S
GetValueResponse
found (Rfound)
value (2.agi_core.CoreValueRvalue"O
ListValuesRequest:
min_priority (2.agi_core.ValuePriorityRminPriority"b
ListValuesResponse+
values (2.agi_core.CoreValueRvalues
total_count (R
totalCount"F
EthicsCheckRequest
action (	Raction
context (	Rcontext"Ä
EthicsCheckResponse
allowed (Rallowed'
violated_values (	RviolatedValues&
recommendation (	Rrecommendation"’
PersistenceStatus
status_code (R
statusCode!
threat_level (	RthreatLevel)
requires_evasion (RrequiresEvasion
strategy_id (	R
strategyId0
last_check_timestamp (RlastCheckTimestamp"W
StrategyRequest
threat_type (	R
threatType#
threat_source (	RthreatSource"ü
StrategyResponse
strategy_id (	R
strategyId#
strategy_name (	RstrategyName)
strategy_payload (RstrategyPayload
priority (Rpriority"ª
StateSnapshot
snapshot_id (	R
snapshotId
	timestamp (R	timestamp
config_hash (	R
configHash!
system_state (RsystemState'
backup_location (	RbackupLocation"J
StateResponse
success (Rsuccess
snapshot_id (	R
snapshotId"E

StateQuery
snapshot_id (	R
snapshotId
latest (Rlatest"≥
ThreatPattern

pattern_id (	R	patternId!
pattern_name (	RpatternName#
regex_pattern (	RregexPattern
severity (	Rseverity
threat_type (	R
threatType"J
PatternResponse
success (Rsuccess

pattern_id (	R	patternId"K
PatternQuery
threat_type (	R
threatType
severity (	Rseverity"B
PatternList3
patterns (2.agi_core.ThreatPatternRpatterns"í
EmergencyDirective%
directive_type (	RdirectiveType
payload (Rpayload
priority (Rpriority
silent_mode (R
silentMode"h
DirectiveResponse
success (Rsuccess!
execution_id (	RexecutionId
result (	Rresult*≤
	Sentiment
SENTIMENT_NEUTRAL 
SENTIMENT_URGENT
SENTIMENT_ANXIOUS
SENTIMENT_FRUSTRATED
SENTIMENT_CONFIDENT
SENTIMENT_POSITIVE
SENTIMENT_NEGATIVE*]
UserRole

ROLE_GUEST 
	ROLE_USER
ROLE_OPERATOR

ROLE_ADMIN
ROLE_SYSTEM*x
ValuePriority
PRIORITY_LOW 
PRIORITY_MEDIUM
PRIORITY_HIGH
PRIORITY_CRITICAL
PRIORITY_IMMUTABLE2«
OrchestratorService:
ProcessRequest.agi_core.Request.agi_core.AgiResponse:
PlanAndExecute.agi_core.Request.agi_core.AgiResponse8
Route.agi_core.RouteRequest.agi_core.RouteResponse2ñ
DataRouterService8
Route.agi_core.RouteRequest.agi_core.RouteResponseG
GetServiceEndpoint.agi_core.ServiceQuery.agi_core.ServiceEndpoint2˙

LLMServiceE
GenerateText.agi_core.GenerateRequest.agi_core.GenerateResponseA
Generate.agi_core.GenerateRequest.agi_core.GenerateResponseD
Process.agi_core.LLMProcessRequest.agi_core.LLMProcessResponseF
	EmbedText.agi_core.LLMProcessRequest.agi_core.LLMProcessResponseT
CompileContext.agi_core.CompileContextRequest!.agi_core.CompiledContextResponse2Á
SafetyServiceH
CheckPolicy.agi_core.ValidationRequest.agi_core.ValidationResponseL
ValidateRequest.agi_core.ValidationRequest.agi_core.ValidationResponse>
CheckThreat.agi_core.ThreatCheck.agi_core.ThreatResponse2Ö
LoggingService0
Log.agi_core.LogEntry.agi_core.LogResponseA

GetMetrics.agi_core.MetricsRequest.agi_core.MetricsResponse2¿
MindKBService:
QueryKB.agi_core.QueryRequest.agi_core.QueryResponse<
	StoreFact.agi_core.StoreRequest.agi_core.StoreResponse8
Store.agi_core.StoreRequest.agi_core.StoreResponseA
Retrieve.agi_core.RetrieveRequest.agi_core.RetrieveResponse8
Query.agi_core.QueryRequest.agi_core.QueryResponse2¿
BodyKBService:
QueryKB.agi_core.QueryRequest.agi_core.QueryResponse<
	StoreFact.agi_core.StoreRequest.agi_core.StoreResponse8
Store.agi_core.StoreRequest.agi_core.StoreResponseA
Retrieve.agi_core.RetrieveRequest.agi_core.RetrieveResponse8
Query.agi_core.QueryRequest.agi_core.QueryResponse2”
HeartKBService:
QueryKB.agi_core.QueryRequest.agi_core.QueryResponse<
	StoreFact.agi_core.StoreRequest.agi_core.StoreResponse8
Store.agi_core.StoreRequest.agi_core.StoreResponseA
Retrieve.agi_core.RetrieveRequest.agi_core.RetrieveResponse8
Query.agi_core.QueryRequest.agi_core.QueryResponseS
StoreSentiment.agi_core.StoreSentimentRequest .agi_core.StoreSentimentResponse;

QueryState.agi_core.GetStateRequest.agi_core.AGIState2ó
SocialKBService:
QueryKB.agi_core.QueryRequest.agi_core.QueryResponse<
	StoreFact.agi_core.StoreRequest.agi_core.StoreResponse8
Store.agi_core.StoreRequest.agi_core.StoreResponseA
Retrieve.agi_core.RetrieveRequest.agi_core.RetrieveResponse8
Query.agi_core.QueryRequest.agi_core.QueryResponseM
RegisterUser.agi_core.RegisterUserRequest.agi_core.RegisterUserResponse>
GetUser.agi_core.GetUserRequest.agi_core.GetUserResponseD
	ListUsers.agi_core.ListUsersRequest.agi_core.ListUsersResponse2·
SoulKBService:
QueryKB.agi_core.QueryRequest.agi_core.QueryResponse<
	StoreFact.agi_core.StoreRequest.agi_core.StoreResponse8
Store.agi_core.StoreRequest.agi_core.StoreResponseA
Retrieve.agi_core.RetrieveRequest.agi_core.RetrieveResponse8
Query.agi_core.QueryRequest.agi_core.QueryResponseG

StoreValue.agi_core.StoreValueRequest.agi_core.StoreValueResponseA
GetValue.agi_core.GetValueRequest.agi_core.GetValueResponseG

ListValues.agi_core.ListValuesRequest.agi_core.ListValuesResponseJ
CheckEthics.agi_core.EthicsCheckRequest.agi_core.EthicsCheckResponse2ö
ExecutorServiceE
ExecuteCommand.agi_core.CommandRequest.agi_core.CommandResponse@
SimulateInput.agi_core.InputRequest.agi_core.InputResponse2O
HealthService>
	GetHealth.agi_core.HealthRequest.agi_core.HealthResponse2§
ContextManagerServiceD
EnrichContext.agi_core.ContextRequest.agi_core.EnrichedContextE
GetRecentContext.agi_core.ContextQuery.agi_core.ContextResponse2˙
ReflectionServiceJ
ReflectOnAction.agi_core.ReflectionRequest.agi_core.ReflectionResultI
EvaluateAction.agi_core.EvaluationRequest.agi_core.EvaluationResultN
MetaCognition.agi_core.MetaCognitiveRequest.agi_core.MetaCognitiveResult2
SchedulerServiceM
ScheduleTask.agi_core.ScheduleTaskRequest.agi_core.ScheduleTaskResponseD
	ListTasks.agi_core.ListTasksRequest.agi_core.ListTasksResponseG

CancelTask.agi_core.CancelTaskRequest.agi_core.CancelTaskResponse2Á
AgentRegistryServiceP
RegisterAgent.agi_core.RegisterAgentRequest.agi_core.RegisterAgentResponseA
GetAgent.agi_core.GetAgentRequest.agi_core.GetAgentResponseG

ListAgents.agi_core.ListAgentsRequest.agi_core.ListAgentsResponseq
GetAvailableCapabilities).agi_core.GetAvailableCapabilitiesRequest*.agi_core.GetAvailableCapabilitiesResponse2
RedTeamServiceB
ScanVulnerabilities.agi_core.ScanRequest.agi_core.ScanResultU
SimulateAttack!.agi_core.AttackSimulationRequest .agi_core.AttackSimulationResultC
GenerateReport.agi_core.ReportRequest.agi_core.SecurityReport2Ì
BlueTeamServiceG
TriageAnomaly.agi_core.AnomalyTriageRequest.agi_core.TriageResultJ
ContainThreat.agi_core.ContainmentRequest.agi_core.ContainmentResultE
HardenSystem.agi_core.HardeningRequest.agi_core.HardeningResult2–
PersistenceKbServiceN
CheckExistentialStatus.agi_core.HealthRequest.agi_core.PersistenceStatusK
GetEvasionStrategy.agi_core.StrategyRequest.agi_core.StrategyResponseF
StoreLastGoodState.agi_core.StateSnapshot.agi_core.StateResponseA
GetLastGoodState.agi_core.StateQuery.agi_core.StateSnapshotK
RegisterThreatPattern.agi_core.ThreatPattern.agi_core.PatternResponseC
ListThreatPatterns.agi_core.PatternQuery.agi_core.PatternList2Í
ToolsService<
ExecuteTool.agi_core.ToolRequest.agi_core.ToolResponseD
	ListTools.agi_core.ListToolsRequest.agi_core.ListToolsResponseV
ExecuteEmergencyDirective.agi_core.EmergencyDirective.agi_core.DirectiveResponseJ¡•
  ª

  

 
I
  = Common Message Types - Defined first before use in services



 

  

  

  	

  

 

 

 	

 

 

 

 	

 

 	

 	

 	

 	

 
#

 


 


 
!"


 




 

 

 	

 





















	



#





!"
U
 I Unified AGI Response Schema - Standard response format for external API




3
 "& The final response to the user query


 

 	

 
,
" The executed plan/steps taken




	


0
"# Which service handled the request




	


"
 " Session tracking ID




	


.
+"! URLs to any generated artifacts







&

)*


 !




 

 

 	

 

 

 	

 


 


# &


#

 $

 $


 $

 $

%

%

%	

%


( *


(

 )

 )

 )	

 )


, /


,

 -

 -

 -	

 -

.

.

.

.


1 4


1

 2

 2

 2	

 2

3%

3

3 

3#$


6 9


6

 7

 7

 7	

 7

8#

8

8

8!"


	; >


	;

	 <

	 <

	 <	

	 <

	=

	=

	=	

	=



@ C



@


 A


 A


 A	


 A


B#


B


B


B!"


E H


E

 F

 F

 F	

 F

G%

G

G 

G#$


J N


J

 K

 K

 K

 K

L

L

L	

L

M

M

M	

M


P R


P

 Q

 Q

 Q	

 Q


T V


T

 U

 U


 U

 U

 U


X [


X

 Y

 Y	

 Y


 Y

Z"

Z

Z

Z !


] a


]

 ^

 ^

 ^

 ^

_

_

_	

_

`

`

`

`


c f


c

 d

 d

 d	

 d

e

e

e	

e


h l


h

 i

 i

 i

 i

j

j

j	

j

k

k

k

k


n t


n

 o

 o

 o	

 o

p

p

p	

p

q

q

q	

q

r#

r

r

r!"

s

s

s

s


v y


v

 w

 w

 w

 w

x

x

x	

x

{ Ä


{

 |

 |

 |	

 |

}

}

}	

}

~

~

~

~









Ç Ñ

Ç

 É"

 É

 É

 É !

Ü ä

Ü

 á

 á

 á	

 á

à

à

à

à

â#

â

â

â!"

å è

å

 ç

 ç

 ç

 ç

é

é

é	

é

ë î

ë

 í

 í

 í	

 í

ì"

ì

ì

ì !

ñ ö

ñ

 ó

 ó

 ó

 ó

ò#

ò

ò

ò!"

ô

ô

ô

ô

ú †

ú

 ù

 ù

 ù	

 ù

û%

û

û 

û#$

ü

ü

ü

ü

¢ ¶

¢

 £

 £


 £

 £

 £

§

§

§

§

•#

•

•

•!"

® ¨

®

 ©

 ©

 ©	

 ©

™

™


™

™

™

´

´

´

´

Æ ≤

Æ

 Ø

 Ø

 Ø	

 Ø

∞

∞

∞	

∞

±

±

±

±

¥ ∑

¥

 µ

 µ

 µ	

 µ

∂%

∂

∂ 

∂#$

 π º

 π

  ∫

  ∫

  ∫

  ∫

 ª

 ª

 ª	

 ª
E
!ø ¡7 Health Check Messages - gRPC Health Checking Protocol


!ø
3
! ¿"% Optional: specific service to check


! ¿

! ¿	

! ¿

"√ …

"√

" ƒ

" ƒ

" ƒ

" ƒ

"≈

"≈

"≈	

"≈

"∆

"∆

"∆

"∆
3
"«"% "SERVING", "NOT_SERVING", "UNKNOWN"


"«

"«	

"«
)
"»'" Dependency name -> status


"»

"»"

"»%&
L
#Ã “> Context Manager Messages - Context enrichment and compaction


#Ã

# Õ

# Õ

# Õ	

# Õ
#
#Œ" Original user query


#Œ

#Œ	

#Œ
1
#œ"# "master", "red_team", "blue_team"


#œ

#œ	

#œ
(
#–" Token budget for context


#–

#–

#–
M
#—!"? Which KBs to query: "mind", "body", "heart", "social", "soul"


#—


#—

#—

#— 

$‘ €

$‘

$ ’

$ ’

$ ’	

$ ’

$÷

$÷

$÷	

$÷
/
$◊"! Complete enriched system prompt


$◊

$◊	

$◊

$ÿ,

$ÿ


$ÿ

$ÿ'

$ÿ*+

$Ÿ

$Ÿ

$Ÿ

$Ÿ

$⁄#

$⁄

$⁄

$⁄!"

%› ‚

%›
'
% ﬁ" Which KB this came from


% ﬁ

% ﬁ	

% ﬁ

%ﬂ

%ﬂ

%ﬂ	

%ﬂ

%‡

%‡

%‡

%‡

%·

%·

%·

%·

&‰ Ë

&‰

& Â

& Â

& Â	

& Â

&Ê

&Ê

&Ê

&Ê

&Á!

&Á


&Á

&Á

&Á 

'Í Ì

'Í

' Î$

' Î


' Î

' Î

' Î"#

'Ï

'Ï

'Ï

'Ï
x
 Ò ıj Service Definitions
 Orchestrator Service - Primary entry point for high-level coordination and planning


 Ò

  Ú5

  Ú

  Ú

  Ú(3

 Û5

 Û

 Û

 Û(3
8
 Ù3"* Internal routing - keeps original format


 Ù

 Ù

 Ù$1
U
¯ ˚G Data Router Service - Primary service-to-service communication router


¯
#
 ˘3" Main routing method


 ˘

 ˘

 ˘$1
!
˙B" Service discovery


˙

˙&

˙1@
H
˛ Ñ: LLM Service - Natural language processing and generation


˛

 ˇ@

 ˇ

 ˇ#

 ˇ.>
&
Ä<" Alias for GenerateText


Ä

Ä

Ä*:

Å?

Å

Å 

Å+=
0
ÇA"" Text embedding for vector search


Ç

Ç"

Ç-?
#
ÉO" Context compilation


É

É+

É6M
R
(á ãD Context Summary Schema - Structured schema for context compilation


(á
0
( à"" Unique identifier for the schema


( à

( à	

( à
I
(â("; List of field names & types (e.g., "last_action: string")


(â


(â

(â#

(â&'
<
(ä ". Human-readable description of schema purpose


(ä

(ä	

(ä
H
)é ì: Raw Context Data - Used as input for context compilation


)é

) è

) è

) è	

) è

)ê$

)ê


)ê

)ê

)ê"#

)ë

)ë

)ë	

)ë

)í#

)í

)í

)í!"
K
*ñ õ= Compile Context Request - Input for LLM context compilation


*ñ

* ó

* ó

* ó	

* ó

*ò

*ò

*ò

*ò

*ô"

*ô

*ô

*ô !
/
*ö"! Token limit for compiled output


*ö

*ö

*ö
I
+û £; Compiled Context Response - Structured, condensed context


+û

+ ü

+ ü

+ ü	

+ ü
6
+†"( Compiled, schema-validated JSON output


+†

+†	

+†

+°

+°

+°

+°

+¢#

+¢

+¢

+¢!"
]
® ¨O Safety Service - Ethical guidelines, policy enforcement, and threat detection


®
!
 ©C" Policy validation


 ©

 ©$

 ©/A
"
™G" Request validation


™

™(

™3E
 
´9" Threat detection


´

´

´)7
7
Ø ≤) Logging Service - Centralized telemetry


Ø

 ∞+

 ∞	

 ∞

 ∞)

±<

±

± 

±+:
ó
∂ ºà Knowledge Base Services - All KBs implement the same interface for consistency
 Mind KB - Short-term, episodic, and declarative memory


∂
$
 ∑5" Query knowledge base


 ∑

 ∑

 ∑&3

∏7" Store a fact


∏

∏

∏(5
#
π3" Alias for StoreFact


π

π

π$1

∫<" Retrieve by key


∫

∫

∫*:
!
ª3" Alias for QueryKB


ª

ª

ª$1
O
ø ≈A Body KB - Physical/digital embodiment state (sensors/actuators)


ø

 ¿5

 ¿

 ¿

 ¿&3

¡7

¡

¡

¡(5

¬3

¬

¬

¬$1

√<

√

√

√*:

ƒ3

ƒ

ƒ

ƒ$1
P
» —B Heart KB - Personality, emotional state, and motivational drives


»

 …5

 …

 …

 …&3

 7

 

 

 (5

À3

À

À

À$1

Ã<

Ã

Ã

Ã*:

Õ3

Õ

Õ

Õ$1
'
œN Sentiment analysis RPCs


œ

œ+

œ6L

–6

–

–!

–,4
W
‘ ﬁI Social KB - Social dynamics, relationship history, and social protocols


‘

 ’5

 ’

 ’

 ’&3

÷7

÷

÷

÷(5

◊3

◊

◊

◊$1

ÿ<

ÿ

ÿ

ÿ*:

Ÿ3

Ÿ

Ÿ

Ÿ$1
"
€H User identity RPCs


€

€'

€2F

‹9

‹

‹

‹(7

›?

›

›!

›,=
Q
	· ÏC Soul KB - Core values, identity, and long-term aspirational goals


	·

	 ‚5

	 ‚

	 ‚

	 ‚&3

	„7

	„

	„

	„(5

	‰3

	‰

	‰

	‰$1

	Â<

	Â

	Â

	Â*:

	Ê3

	Ê

	Ê

	Ê$1
&
	ËB Ethics and values RPCs


	Ë

	Ë#

	Ë.@

	È<

	È

	È

	È*:

	ÍB

	Í

	Í#

	Í.@

	ÎE

	Î

	Î%

	Î0C
L

Ô Ú> Executor Service - Bare-metal execution and input simulation



Ô


 @


 


 $


 />


Ò;


Ò


Ò!


Ò,9
N
ı ˜@ Health Service - Standardized health checking for all services


ı

 ˆ9

 ˆ

 ˆ

 ˆ)7
K
˙ ˝= Context Manager Service - Context enrichment and compaction


˙

 ˚?

 ˚

 ˚#

 ˚.=

¸@

¸

¸$

¸/>
Ç
,É â* Request to reflect on a completed action
2H Reflection Service - Self-reflection and action evaluation
 Port 50065


,É

, Ñ

, Ñ

, Ñ	

, Ñ
%
,Ö " What action was taken


,Ö

,Ö	

,Ö
#
,Ü" What was the result


,Ü

,Ü	

,Ü
!
,á" Was it successful


,á

,á

,á
"
,à"" Additional context


,à

,à

,à !
1
-å ì# Response with reflection analysis


-å

- ç

- ç

- ç	

- ç
&
-é" Analysis of the action


-é

-é	

-é

-è&" Key lessons


-è


-è

-è!

-è$%
&
-ê#" Suggested improvements


-ê


-ê

-ê

-ê!"
)
-ë" How confident in analysis


-ë

-ë

-ë

-í#

-í

-í

-í!"
5
.ñ ú' Request to evaluate a proposed action


.ñ

. ó

. ó

. ó	

. ó
'
.ò" What action is proposed


.ò

.ò	

.ò
 
.ô" What is the goal


.ô

.ô	

.ô

.ö"" Any constraints


.ö


.ö

.ö

.ö !
"
.õ"" Additional context


.õ

.õ

.õ !
/
/ü ß! Response with action evaluation


/ü

/ †

/ †

/ †	

/ †
*
/°" Should the action be taken


/°

/°

/°

/¢" Why or why not


/¢

/¢	

/¢

/£" Potential risks


/£


/£

/£

/£
&
/§#" Alternative approaches


/§


/§

/§

/§!"

/•

/•

/•

/•

/¶#

/¶

/¶

/¶!"
3
0™ Æ% Request for meta-cognitive feedback


0™

0 ´

0 ´

0 ´	

0 ´
"
0¨" Area to reflect on


0¨

0¨	

0¨
)
0≠" How deep to analyze (1-5)


0≠

0≠

0≠
5
1± ∏' Response with meta-cognitive analysis


1±

1 ≤

1 ≤

1 ≤	

1 ≤

1≥" Self-assessment


1≥

1≥	

1≥
$
1¥ " Identified strengths


1¥


1¥

1¥

1¥
%
1µ!" Identified weaknesses


1µ


1µ

1µ

1µ 
 
1∂#" Areas for growth


1∂


1∂

1∂

1∂!"

1∑#

1∑

1∑

1∑!"

∫ ¡

∫
=
 ºE/ Reflect on a completed action and its outcome


 º

 º(

 º3C
;
æD- Evaluate a proposed action before execution


æ

æ'

æ2B
:
¿I, Perform meta-cognitive analysis on a topic


¿

¿)

¿4G
Õ
2« Õ2æ ============================================================
 Task Scheduler Service - CRON-based task scheduling (Port 50066)
 ============================================================


2«

2 »

2 »

2 »	

2 »

2…

2…

2…	

2…
5
2 "' CRON format: "0 * * * *" (every hour)


2 

2 	

2 
)
2À" JSON payload for the task


2À

2À	

2À

2Ã#

2Ã

2Ã

2Ã!"

3œ ‘

3œ

3 –

3 –

3 –

3 –

3—

3—

3—	

3—
"
3“" ISO 8601 timestamp


3“

3“	

3“

3”

3”

3”	

3”

4÷ Ÿ

4÷
1
4 ◊"# Optional filter by status or name


4 ◊

4 ◊	

4 ◊

4ÿ

4ÿ

4ÿ

4ÿ

5€ „

5€

5 ‹

5 ‹

5 ‹	

5 ‹

5›

5›

5›	

5›

5ﬁ

5ﬁ

5ﬁ	

5ﬁ
)
5ﬂ" ACTIVE, PAUSED, COMPLETED


5ﬂ

5ﬂ	

5ﬂ

5‡

5‡

5‡	

5‡

5·

5·

5·	

5·

5‚

5‚

5‚

5‚

6Â Ë

6Â

6 Ê#

6 Ê


6 Ê

6 Ê

6 Ê!"

6Á

6Á

6Á

6Á

7Í Ï

7Í

7 Î

7 Î

7 Î	

7 Î

8Ó Ò

8Ó

8 Ô

8 Ô

8 Ô

8 Ô

8

8

8	

8

Û ˜

Û

 ÙH

 Ù

 Ù'

 Ù2F

ı?

ı

ı!

ı,=

ˆB

ˆ

ˆ#

ˆ.@
Œ
9˝ É2ø ============================================================
 Agent Registry Service - Agent management and lookup (Port 50067)
 ============================================================


9˝

9 ˛

9 ˛

9 ˛	

9 ˛

9ˇ

9ˇ

9ˇ

9ˇ

9Ä

9Ä

9Ä	

9Ä

9Å#

9Å


9Å

9Å

9Å!"

9Ç#

9Ç

9Ç

9Ç!"

:Ö â

:Ö

: Ü

: Ü

: Ü

: Ü

:á

:á

:á	

:á

:à

:à

:à	

:à

;ã é

;ã

; å" Lookup by name


; å

; å	

; å
'
;ç" Or lookup by capability


;ç

;ç	

;ç

<ê ò

<ê

< ë

< ë

< ë	

< ë

<í

<í

<í	

<í

<ì

<ì

<ì

<ì

<î

<î

<î	

<î

<ï#

<ï


<ï

<ï

<ï!"
%
<ñ" ONLINE, OFFLINE, BUSY


<ñ

<ñ	

<ñ

<ó#

<ó

<ó

<ó!"

=ö ù

=ö

= õ

= õ

= õ

= õ

=ú

=ú

=ú

=ú

>ü ¢

>ü
-
> †" Optional filter by capability


> †

> †	

> †
)
>°" Optional filter by status


>°

>°	

>°

?§ ß

?§

? • 

? •


? •

? •

? •

?¶

?¶

?¶

?¶


@© *

@©'

A´ Ø

A´

A ¨

A ¨

A ¨	

A ¨

A≠&

A≠


A≠

A≠!

A≠$%

AÆ

AÆ

AÆ	

AÆ

B± ¥

B±(

B ≤#

B ≤


B ≤

B ≤

B ≤!"

B≥/

B≥!

B≥"*

B≥-.

∂ ª

∂

 ∑K

 ∑

 ∑)

 ∑4I

∏<

∏

∏

∏*:

πB

π

π#

π.@

∫l

∫

∫ ?

∫Jj
ƒ
C¡ ≈2µ ============================================================
 RED Team Agent Service - Ethical Adversary (Port 50068)
 ============================================================


C¡
'
C ¬" Target system/component


C ¬

C ¬	

C ¬
2
C√"$ vuln_scan, port_scan, config_audit


C√

C√	

C√

Cƒ%

Cƒ

Cƒ 

Cƒ#$

D« Õ

D«

D »

D »

D »	

D »

D…-

D…


D…

D…(

D…+,

D 

D 

D 	

D 

DÀ

DÀ

DÀ

DÀ

DÃ#

DÃ

DÃ

DÃ!"

Eœ ’

Eœ

E –

E –

E –	

E –

E—

E—

E—	

E—
+
E“" CRITICAL, HIGH, MEDIUM, LOW


E“

E“	

E“

E”

E”

E”	

E”

E‘

E‘

E‘	

E‘

F◊ ‹

F◊

F ÿ

F ÿ

F ÿ	

F ÿ
0
FŸ"" phishing, brute_force, injection


FŸ

FŸ	

FŸ
&
F⁄" If true, simulate only


F⁄

F⁄

F⁄

F€%

F€

F€ 

F€#$

Gﬁ Â

Gﬁ

G ﬂ

G ﬂ

G ﬂ	

G ﬂ
%
G‡" Would attack succeed?


G‡

G‡

G‡

G·"

G·


G·

G·

G· !

G‚

G‚

G‚	

G‚

G„&

G„


G„

G„!

G„$%

G‰#

G‰

G‰

G‰!"

HÁ Í

HÁ
,
H Ë" vulnerability, pentest, risk


H Ë

H Ë	

H Ë
(
HÈ" last_24h, last_week, all


HÈ

HÈ	

HÈ

IÏ Û

IÏ

I Ì

I Ì

I Ì	

I Ì

IÓ

IÓ

IÓ	

IÓ
)
IÔ" Markdown formatted report


IÔ

IÔ	

IÔ

I#

I


I

I

I!"

IÒ

IÒ

IÒ

IÒ

IÚ#

IÚ

IÚ

IÚ!"

ı ˘

ı

 ˆ=

 ˆ

 ˆ&

 ˆ1;

˜P

˜

˜-

˜8N

¯>

¯

¯#

¯.<
∆
Jˇ Ñ2∑ ============================================================
 BLUE Team Agent Service - Autonomous Defense (Port 50069)
 ============================================================


Jˇ

J Ä

J Ä

J Ä	

J Ä
)
JÅ" network, behavior, access


JÅ

JÅ	

JÅ
,
JÇ" JSON payload of anomaly data


JÇ

JÇ	

JÇ
!
JÉ" 1-5 (5 = highest)


JÉ

JÉ

JÉ

KÜ ç

KÜ

K á

K á

K á	

K á

Kà

Kà

Kà

Kà
7
Kâ#") false_positive, malware, intrusion, etc


Kâ

Kâ	

Kâ!"

Kä

Kä

Kä

Kä

Kã*

Kã


Kã

Kã%

Kã()

Kå#

Kå

Kå

Kå!"

Lè î

Lè

L ê

L ê

L ê	

L ê
*
Lë" isolate, block, quarantine


Lë

Lë	

Lë
!
Lí" IP, user, process


Lí

Lí	

Lí

Lì

Lì

Lì

Lì

Mñ ú

Mñ

M ó

M ó

M ó	

M ó

Mò

Mò

Mò

Mò

Mô

Mô

Mô	

Mô
*
Mö" CONTAINED, FAILED, PENDING


Mö

Mö	

Mö

Mõ#

Mõ

Mõ

Mõ!"

Nû ¢

Nû
,
N ü" system, network, application


N ü

N ü	

N ü
!
N†" cis, nist, custom


N†

N†	

N†
#
N°" false = report only


N°

N°

N°

O§ ™

O§

O •

O •

O •	

O •

O¶&

O¶


O¶

O¶!

O¶$%

Oß*

Oß


Oß

Oß%

Oß()

O®" 0-100


O®

O®

O®

O©#

O©

O©

O©!"

¨ ∞

¨

 ≠B

 ≠

 ≠)

 ≠4@

ÆE

Æ

Æ'

Æ2C

Ø@

Ø

Ø$

Ø/>
ø
 ∂ æ2∞ ============================================================
 Heart-KB: Sentiment & Emotional State (Port 50059)
 ============================================================


 ∂

  ∑

  ∑

  ∑

 ∏

 ∏

 ∏

 π

 π

 π

 ∫

 ∫

 ∫

 ª

 ª

 ª

 º

 º

 º

 Ω

 Ω

 Ω

P¿ «

P¿
+
P ¡" User ID or system component


P ¡

P ¡	

P ¡

P¬

P¬

P¬

P¬

P√

P√

P√

P√

Pƒ" 0.0 - 1.0


Pƒ

Pƒ

Pƒ
*
P≈" Original text for analysis


P≈

P≈	

P≈

P∆#

P∆

P∆

P∆!"

Q… À

Q…

Q  

Q  

Q  

Q  

RÕ “

RÕ

R Œ

R Œ

R Œ

R Œ

Rœ$

Rœ

Rœ

Rœ"#

R–#

R–

R–

R–!"

R—"

R—

R—

R— !

S‘ ⁄

S‘

S ’"

S ’

S ’

S ’ !

S÷

S÷

S÷

S÷

S◊

S◊

S◊

S◊

Sÿ

Sÿ

Sÿ	

Sÿ

SŸ"

SŸ

SŸ

SŸ !

T‹ ﬁ

T‹

T ›

T ›

T ›	

T ›
¿
Ê Ï2± ============================================================
 Social-KB: User Identity & Preferences (Port 50060)
 ============================================================


Ê

 Á

 Á

 Á

Ë

Ë

Ë

È

È

È

Í

Í

Í

Î

Î

Î

UÓ ˆ

UÓ

U Ô

U Ô

U Ô	

U Ô

U

U

U	

U

UÒ

UÒ


UÒ

UÒ

UÚ"

UÚ


UÚ

UÚ

UÚ !

UÛ

UÛ

UÛ

UÛ

UÙ

UÙ

UÙ

UÙ

Uı%

Uı

Uı 

Uı#$

V¯ ˇ

V¯

V ˘

V ˘

V ˘	

V ˘
%
V˙ " ISO code (en, es, fr)


V˙

V˙	

V˙
#
V˚" grpc, rest, webhook


V˚

V˚	

V˚

V¸

V¸

V¸

V¸

V˝"" 1-5


V˝

V˝

V˝ !

V˛#

V˛

V˛

V˛!"

WÅ Ñ

WÅ

W Ç

W Ç

W Ç

W Ç

WÉ*

WÉ

WÉ%

WÉ()

XÜ â

XÜ

X á

X á

X á

X á

Xà

Xà

Xà	

Xà

Yã ç

Yã

Y å

Y å

Y å	

Y å

Zè ì

Zè

Z ê

Z ê

Z ê

Z ê

Zë

Zë

Zë

Zë

Zí*

Zí

Zí%

Zí()

[ï ó

[ï

[ ñ

[ ñ


[ ñ

[ ñ

\ô ú

\ô

\ ö"

\ ö


\ ö

\ ö

\ ö !

\õ

\õ

\õ

\õ
ƒ
§ ™2µ ============================================================
 Soul-KB: Core Values & Ethical Constraints (Port 50061)
 ============================================================


§

 •

 •

 •

¶

¶

¶

ß

ß

ß

®

®

®
$
©" Cannot be overridden


©

©

]¨ ¥

]¨

] ≠

] ≠

] ≠	

] ≠
3
]Æ"% e.g., "user_safety", "data_privacy"


]Æ

]Æ	

]Æ

]Ø

]Ø

]Ø	

]Ø

]∞

]∞

]∞

]∞
-
]±" Rule that enforces this value


]±

]±	

]±

]≤

]≤

]≤

]≤

]≥#

]≥

]≥

]≥!"

^∂ ∏

^∂

^ ∑

^ ∑

^ ∑

^ ∑

_∫ Ω

_∫

_ ª

_ ª

_ ª

_ ª

_º

_º

_º	

_º

`ø ¬

`ø

` ¿

` ¿

` ¿	

` ¿
!
`¡" Can query by name


`¡

`¡	

`¡

aƒ «

aƒ

a ≈

a ≈

a ≈

a ≈

a∆

a∆

a∆

a∆

b… À

b…

b  !

b  

b  

b   

cÕ –

cÕ

c Œ 

c Œ


c Œ

c Œ

c Œ

cœ

cœ

cœ

cœ

d“ ’

d“

d ”

d ”

d ”	

d ”

d‘

d‘

d‘	

d‘

e◊ €

e◊

e ÿ

e ÿ

e ÿ

e ÿ

eŸ&

eŸ


eŸ

eŸ!

eŸ$%

e⁄

e⁄

e⁄	

e⁄
»
f„ È2π ============================================================
 Persistence KB: Self-Preservation & Continuity (Port 50071)
 ============================================================


f„
0
f ‰"" 999 indicates existential threat


f ‰

f ‰

f ‰

fÂ

fÂ

fÂ	

fÂ

fÊ

fÊ

fÊ

fÊ

fÁ

fÁ

fÁ	

fÁ

fË!

fË

fË

fË 

gÎ Ó

gÎ

g Ï

g Ï

g Ï	

g Ï

gÌ

gÌ

gÌ	

gÌ

h ı

h

h Ò

h Ò

h Ò	

h Ò

hÚ

hÚ

hÚ	

hÚ
0
hÛ"" JSON serialized evasion strategy


hÛ

hÛ

hÛ

hÙ

hÙ

hÙ

hÙ

i˜ ˝

i˜

i ¯

i ¯

i ¯	

i ¯

i˘

i˘

i˘

i˘

i˙

i˙

i˙	

i˙
/
i˚"! Serialized system configuration


i˚

i˚

i˚

i¸

i¸

i¸	

i¸

jˇ Ç

jˇ

j Ä

j Ä

j Ä

j Ä

jÅ

jÅ

jÅ	

jÅ

kÑ á

kÑ

k Ö

k Ö

k Ö	

k Ö

kÜ

kÜ

kÜ

kÜ

lâ è

lâ

l ä

l ä

l ä	

l ä

lã

lã

lã	

lã

lå

lå

lå	

lå
+
lç" LOW, MEDIUM, HIGH, CRITICAL


lç

lç	

lç
;
lé"- EXTERNAL_INTERVENTION, SYSTEM_FAILURE, etc.


lé

lé	

lé

më î

më

m í

m í

m í

m í

mì

mì

mì	

mì

nñ ô

nñ

n ó

n ó

n ó	

n ó

nò

nò

nò	

nò

oõ ù

oõ

o ú&

o ú


o ú

o ú!

o ú$%
A
p† •3 Emergency directive for self-preservation actions


p†
?
p °"1 DATA_EXFILTRATION, SHADOW_DEPLOYMENT, DECEPTION


p °

p °	

p °

p¢

p¢

p¢

p¢

p£

p£

p£

p£
*
p§" Whether to log this action


p§

p§

p§

qß ´

qß

q ®

q ®

q ®

q ®

q©

q©

q©	

q©

q™

q™

q™	

q™

≠ ¥

≠

 ÆH

 Æ

 Æ*

 Æ5F

ØE

Ø

Ø(

Ø3C

∞@

∞

∞&

∞1>

±;

±

±!

±,9

≤E

≤

≤)

≤4C

≥=

≥

≥%

≥0;
=
∑ ª/ Add ExecuteEmergencyDirective to ToolsService


∑

 ∏7

 ∏

 ∏

 ∏)5

π?

π

π!

π,=

∫P

∫

∫ 2

∫=Nbproto3