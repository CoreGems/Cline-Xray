import os
from dotenv import load_dotenv
from jira import JIRA

# Load environment variables from .env file
load_dotenv()

jira = JIRA(
    os.getenv('JIRA_URL'),
    basic_auth=(os.getenv('JIRA_EMAIL'), os.getenv('JIRA_API_TOKEN'))
)

# Get current user
current_user = jira.current_user()
print(f"Listing Jira issues for: {current_user}\n")

# Query for issues assigned to current user
# You can modify the JQL query as needed
jql_query = f'assignee = currentUser() ORDER BY updated DESC'

# Get all issues (max 100, adjust maxResults as needed)
issues = jira.search_issues(jql_query, maxResults=100)

print(f"Total issues found: {len(issues)}\n")
print("-" * 80)

for issue in issues:
    status = issue.fields.status.name
    summary = issue.fields.summary
    key = issue.key
    
    print(f"[{key}] {summary}")
    print(f"  Status: {status}")
    print(f"  Updated: {issue.fields.updated}")
    print("-" * 80)
