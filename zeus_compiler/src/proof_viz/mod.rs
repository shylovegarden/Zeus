// Proof Visualization Module
// Generates interactive HTML/SVG visualizations of Zeus proofs

use crate::formal_verifier::FormalVerifier;
use crate::zir::ZirReport;
use std::collections::HashMap;

/// Proof visualization generator
pub struct ProofViz {
    title: String,
    nodes: Vec<ProofNode>,
    edges: Vec<ProofEdge>,
    next_id: usize,
}

#[derive(Debug, Clone)]
struct ProofNode {
    id: String,
    label: String,
    node_type: NodeType,
    status: ProofStatus,
    details: String,
    x: f64,
    y: f64,
}

#[derive(Debug, Clone)]
struct ProofEdge {
    from: String,
    to: String,
    label: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
enum NodeType {
    Function,
    Contract,
    Verification,
    Proof,
    Constraint,
    Variable,
    Assertion,
    Z3Query,
}

#[derive(Debug, Clone)]
enum ProofStatus {
    Proven,
    Unproven,
    InProgress,
    Failed,
    Assumed,
}

impl ProofViz {
    pub fn new(title: &str) -> Self {
        ProofViz {
            title: title.to_string(),
            nodes: Vec::new(),
            edges: Vec::new(),
            next_id: 0,
        }
    }
    
    /// Generate interactive HTML visualization
    pub fn generate_html(&self) -> String {
        let mut html = String::new();
        
        html.push_str("<!DOCTYPE html>\n");
        html.push_str("<html>\n<head>\n");
        html.push_str(&format!("<title>{}</title>\n", self.title));
        html.push_str("<style>\n");
        html.push_str(self.css_styles());
        html.push_str("</style>\n");
        html.push_str("</head>\n<body>\n");
        
        // Header
        html.push_str(&format!("<h1>{}</h1>\n", self.title));
        html.push_str("<div class='proof-container'>\n");
        
        // Summary panel
        html.push_str("<div class='summary-panel'>\n");
        html.push_str(&self.generate_summary());
        html.push_str("</div>\n");
        
        // Proof tree SVG
        html.push_str("<div class='proof-tree'>\n");
        html.push_str("<svg width='900' height='700' viewBox='0 0 900 700'>\n");
        html.push_str(&self.generate_svg_tree());
        html.push_str("</svg>\n");
        html.push_str("</div>\n");
        
        // Details panel
        html.push_str("<div class='details-panel' id='details'>\n");
        html.push_str("<h2>Proof Details</h2>\n");
        html.push_str("<p>Click on a node to view details</p>\n");
        html.push_str("</div>\n");
        
        // JavaScript for interactivity
        html.push_str("<script>\n");
        html.push_str(self.javascript_code());
        html.push_str("</script>\n");
        
        html.push_str("</div>\n");
        html.push_str("</body>\n</html>\n");
        
        html
    }
    
    fn css_styles(&self) -> &'static str {
        r#"
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #f5f5f5; margin: 0; padding: 20px; }
        h1 { color: #333; border-bottom: 3px solid #4caf50; padding-bottom: 10px; margin-top: 0; }
        .proof-container { display: grid; grid-template-columns: 250px 1fr 300px; gap: 20px; max-width: 1400px; margin: 0 auto; }
        .summary-panel { background: white; padding: 20px; border-radius: 8px; box-shadow: 0 2px 8px rgba(0,0,0,0.1); }
        .proof-tree { background: white; padding: 20px; border-radius: 8px; box-shadow: 0 2px 8px rgba(0,0,0,0.1); overflow: auto; }
        .details-panel { background: white; padding: 20px; border-radius: 8px; box-shadow: 0 2px 8px rgba(0,0,0,0.1); }
        .node { cursor: pointer; transition: all 0.3s; }
        .node:hover { filter: brightness(1.1); transform: scale(1.02); }
        .node.proven { fill: #4caf50; stroke: #2e7d32; }
        .node.unproven { fill: #ff9800; stroke: #ef6c00; }
        .node.failed { fill: #f44336; stroke: #c62828; }
        .node.assumed { fill: #9e9e9e; stroke: #616161; }
        .edge { stroke: #666; stroke-width: 2; fill: none; }
        .node-label { font-size: 12px; font-weight: bold; fill: white; text-anchor: middle; pointer-events: none; }
        .stat-box { margin: 10px 0; padding: 10px; border-radius: 4px; }
        .stat-box.proven { background: #e8f5e9; border-left: 4px solid #4caf50; }
        .stat-box.unproven { background: #fff3e0; border-left: 4px solid #ff9800; }
        .stat-box.failed { background: #ffebee; border-left: 4px solid #f44336; }
        .detail-item { margin: 10px 0; padding: 10px; background: #f5f5f5; border-radius: 4px; }
        "#
    }
    
    fn javascript_code(&self) -> String {
        let mut details_json = String::new();
        details_json.push_str("{");
        for (i, node) in self.nodes.iter().enumerate() {
            if i > 0 { details_json.push(','); }
            details_json.push_str(&format!(
                "'{}': '<div class=\"detail-item\"><h3>{}</h3><p>Type: {:?}</p><p>Status: {:?}</p><p>{}</p></div>'",
                node.id, node.label, node.node_type, node.status, node.details
            ));
        }
        details_json.push_str("}");
        
        format!(
            r#"
            const nodeDetails = {};
            document.querySelectorAll('.node').forEach(node => {{
                node.addEventListener('click', function() {{
                    const id = this.getAttribute('data-id');
                    const details = nodeDetails[id] || '<p>No details available</p>';
                    document.getElementById('details').innerHTML = '<h2>Proof Details</h2>' + details;
                }});
            }});
            "#,
            details_json
        )
    }
    
    fn generate_svg_tree(&self) -> String {
        let mut svg = String::new();
        
        // Draw edges first (behind nodes)
        for edge in &self.edges {
            if let (Some(from_pos), Some(to_pos)) = (self.get_node_pos(&edge.from), self.get_node_pos(&edge.to)) {
                svg.push_str(&format!(
                    "<line x1='{:.0}' y1='{:.0}' x2='{:.0}' y2='{:.0}' class='edge' />\n",
                    from_pos.0, from_pos.1, to_pos.0, to_pos.1
                ));
            }
        }
        
        // Draw nodes
        for node in &self.nodes {
            let color_class = match node.status {
                ProofStatus::Proven => "proven",
                ProofStatus::Unproven => "unproven",
                ProofStatus::Failed => "failed",
                ProofStatus::Assumed => "assumed",
                _ => "unproven",
            };
            
            // Node rectangle
            svg.push_str(&format!(
                "<rect x='{:.0}' y='{:.0}' width='120' height='50' rx='8' class='node {}' data-id='{}' />\n",
                node.x - 60.0, node.y - 25.0, color_class, node.id
            ));
            
            // Node label
            svg.push_str(&format!(
                "<text x='{:.0}' y='{:.0}' class='node-label'>{}</text>\n",
                node.x, node.y + 5.0, node.label
            ));
        }
        
        svg
    }
    
    fn get_node_pos(&self, id: &str) -> Option<(f64, f64)> {
        self.nodes.iter().find(|n| n.id == id).map(|n| (n.x, n.y))
    }
    
    fn generate_summary(&self) -> String {
        let proven = self.nodes.iter().filter(|n| matches!(n.status, ProofStatus::Proven)).count();
        let unproven = self.nodes.iter().filter(|n| matches!(n.status, ProofStatus::Unproven)).count();
        let failed = self.nodes.iter().filter(|n| matches!(n.status, ProofStatus::Failed)).count();
        
        format!(
            r#"
            <h2>Summary</h2>
            <div class='stat-box proven'><strong>{}</strong> Proven</div>
            <div class='stat-box unproven'><strong>{}</strong> Unproven</div>
            <div class='stat-box failed'><strong>{}</strong> Failed</div>
            <div style='margin-top: 20px; font-size: 12px; color: #666;'>
                Total: {} verification conditions
            </div>
            "#,
            proven, unproven, failed, self.nodes.len()
        )
    }
    
    /// Build visualization from ZIR report
    pub fn from_zir(report: &ZirReport) -> Self {
        let mut viz = ProofViz::new("Zeus Proof Visualization");
        
        // Add function nodes
        for (i, pf) in report.per_fn.iter().enumerate() {
            let status = if pf.constant_time && pf.deterministic {
                ProofStatus::Proven
            } else if pf.constant_time || pf.deterministic {
                ProofStatus::InProgress
            } else {
                ProofStatus::Unproven
            };
            
            let x = 150.0 + (i as f64 % 3.0) * 200.0;
            let y = 100.0 + (i as f64 / 3.0).floor() * 120.0;
            
            viz.add_node(ProofNode {
                id: format!("fn_{}", i),
                label: pf.name.clone(),
                node_type: NodeType::Function,
                status,
                details: format!(
                    "Constant-time: {}<br>Deterministic: {}<br>Reaches extern: {}",
                    pf.constant_time, pf.deterministic, pf.reaches_extern
                ),
                x,
                y,
            });
        }
        
        viz
    }
    
    pub fn add_node(&mut self, node: ProofNode) {
        self.nodes.push(node);
    }
    
    pub fn add_edge(&mut self, from: &str, to: &str, label: &str) {
        self.edges.push(ProofEdge {
            from: from.to_string(),
            to: to.to_string(),
            label: label.to_string(),
        });
    }
    
    /// Save visualization to file
    pub fn save(&self, path: &str) -> Result<(), std::io::Error> {
        std::fs::write(path, self.generate_html())
    }
}

/// CLI command implementation
pub fn cmd_proof_viz(file_path: &str, output: Option<&str>) {
    // Parse and analyze
    let src = std::fs::read_to_string(file_path).expect("Cannot read file");
    let lx = crate::lexer::Lexer::new(&src);
    let mut parser = crate::parser::Parser::new(lx);
    let program = parser.parse_program();
    
    // Generate ZIR report
    let zir_report = crate::zir::lower_and_analyze(&program);
    
    // Generate visualization
    let viz = ProofViz::from_zir(&zir_report);
    
    let output_path = output.unwrap_or("proof_viz.html");
    viz.save(output_path).expect("Failed to save visualization");
    
    println!("✅ Proof visualization saved to {}", output_path);
    println!("   Open in browser: file://{}/{}", std::env::current_dir().unwrap().display(), output_path);
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_proof_viz_creation() {
        let viz = ProofViz::new("Test");
        let html = viz.generate_html();
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("Test"));
    }
}
