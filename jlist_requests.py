import os
import requests
from requests.auth import HTTPBasicAuth
from dotenv import load_dotenv

# Load environment variables from .env file
load_dotenv()

# Jira configuration from environment
JIRA_URL = os.getenv('JIRA_URL')
JIRA_EMAIL = os.getenv('JIRA_EMAIL')
JIRA_API_TOKEN = os.getenv('JIRA_API_TOKEN')

# Set up authentication
auth = HTTPBasicAuth(JIRA_EMAIL, JIRA_API_TOKEN)

# Common headers for Jira API
headers = {
    'Accept': 'application/json',
    'Content-Type': 'application/json'
}

def get_current_user():
    """Get the current authenticated user"""
    url = f"{JIRA_URL}/rest/api/3/myself"
    response = requests.get(url, headers=headers, auth=auth)
    
    if response.status_code == 200:
        return response.json()
    else:
        print(f"Error getting current user: {response.status_code} {response.text}")
        return None

def search_issues(jql, max_results=100):
    """Search for issues using JQL (using new /search/jql endpoint)"""
    url = f"{JIRA_URL}/rest/api/3/search/jql"
    
    params = {
        'jql': jql,
        'maxResults': max_results,
        'fields': 'key,summary,status,updated,assignee,priority,issuetype'
    }
    
    response = requests.get(url, headers=headers, auth=auth, params=params)
    
    print(f"Response: {response.status_code} {response.reason}")
    
    if response.status_code == 200:
        return response.json()
    else:
        print(f"Error searching issues: {response.text}")
        return None

def main():
    # Get current user
    user = get_current_user()
    if user:
        print(f"Listing Jira issues for: {user.get('displayName', user.get('emailAddress'))}\n")
    
    # Query for issues assigned to current user
    jql_query = 'assignee = currentUser() ORDER BY updated DESC'
    
    # Get issues
    result = search_issues(jql_query, max_results=100)
    
    if result:
        issues = result.get('issues', [])
        total = result.get('total', len(issues))
        
        print(f"Total issues found: {total} (showing {len(issues)})\n")
        print("-" * 80)
        
        for issue in issues:
            key = issue['key']
            fields = issue['fields']
            summary = fields.get('summary', 'No summary')
            status = fields.get('status', {}).get('name', 'Unknown')
            updated = fields.get('updated', 'Unknown')
            priority = fields.get('priority', {})
            priority_name = priority.get('name', 'None') if priority else 'None'
            issue_type = fields.get('issuetype', {}).get('name', 'Unknown')
            
            print(f"[{key}] {summary}")
            print(f"  Type: {issue_type} | Status: {status} | Priority: {priority_name}")
            print(f"  Updated: {updated}")
            print("-" * 80)

if __name__ == "__main__":
    main()
