{{/*
Expand the name of the chart.
*/}}
{{- define "kubizone.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a default fully qualified app name.
We truncate at 63 chars because some Kubernetes name fields are limited to this (by the DNS naming spec).
If release name contains chart name it will be used as a full name.
*/}}
{{- define "kubizone.fullname" -}}
{{- if .Values.fullnameOverride }}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- $name := default "kubizone" .Values.nameOverride }}
{{- if contains $name .Release.Name }}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}
{{- end }}

{{/*
Create a default fully qualified app name.
We truncate at 63 chars because some Kubernetes name fields are limited to this (by the DNS naming spec).
If release name contains chart name it will be used as a full name.
*/}}
{{- define "zonefile.fullname" -}}
{{- if .Values.fullnameOverride }}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- $name := default "zonefile" .Values.nameOverride }}
{{- if contains $name .Release.Name }}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}
{{- end }}

{{/*
Create chart name and version as used by the chart label.
*/}}
{{- define "kubizone.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Common labels
*/}}
{{- define "kubizone.labels" -}}
helm.sh/chart: {{ include "kubizone.chart" . }}
{{ include "kubizone.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}
{{- define "zonefile.labels" -}}
helm.sh/chart: {{ include "kubizone.chart" . }}
{{ include "zonefile.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "kubizone.selectorLabels" -}}
app.kubernetes.io/name: {{ include "kubizone.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}
{{- define "zonefile.selectorLabels" -}}
app.kubernetes.io/name: {{ include "kubizone.name" . }}-zonefile
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{/*
Create the name of the service account to use
*/}}
{{- define "kubizone.serviceAccountName" -}}
{{- if .Values.kubizone.serviceAccount.create }}
{{- default (include "kubizone.fullname" .) .Values.kubizone.serviceAccount.name }}
{{- else }}
{{- default "default" .Values.kubizone.serviceAccount.name }}
{{- end }}
{{- end }}

{{/*
Create the name of the service account to use
*/}}
{{- define "zonefile.serviceAccountName" -}}
{{- if .Values.zonefile.serviceAccount.create }}
{{- default (include "zonefile.fullname" .) .Values.zonefile.serviceAccount.name }}
{{- else }}
{{- default "default" .Values.zonefile.serviceAccount.name }}
{{- end }}
{{- end }}
