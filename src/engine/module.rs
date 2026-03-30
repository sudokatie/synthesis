//! Module trait and connection system for modular routing

use std::collections::HashMap;

/// Port identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PortId(pub u32);

/// Input port definition
#[derive(Debug, Clone)]
pub struct InputPort {
    pub id: PortId,
    pub name: &'static str,
    pub default: f32,
}

/// Output port definition
#[derive(Debug, Clone)]
pub struct OutputPort {
    pub id: PortId,
    pub name: &'static str,
}

/// Parameter definition
#[derive(Debug, Clone)]
pub struct Parameter {
    pub id: PortId,
    pub name: &'static str,
    pub min: f32,
    pub max: f32,
    pub default: f32,
    pub value: f32,
}

impl Parameter {
    pub fn new(id: u32, name: &'static str, min: f32, max: f32, default: f32) -> Self {
        Self {
            id: PortId(id),
            name,
            min,
            max,
            default,
            value: default,
        }
    }

    pub fn set(&mut self, value: f32) {
        self.value = value.clamp(self.min, self.max);
    }
}

/// Processing context passed to modules
#[derive(Debug, Clone)]
pub struct ModuleContext<'a> {
    pub sample_rate: u32,
    pub buffer_size: usize,
    pub inputs: &'a HashMap<PortId, f32>,
    pub bpm: f32,
}

/// Core module trait for all DSP components
pub trait Module: Send {
    /// Process audio and return output buffer
    fn process(&mut self, context: &ModuleContext, buffer: &mut [f32]);
    
    /// Get input port definitions
    fn inputs(&self) -> &[InputPort];
    
    /// Get output port definitions  
    fn outputs(&self) -> &[OutputPort];
    
    /// Get parameter definitions
    fn parameters(&self) -> &[Parameter];
    
    /// Get mutable parameter access
    fn parameters_mut(&mut self) -> &mut [Parameter];
    
    /// Set parameter value by id
    fn set_parameter(&mut self, id: PortId, value: f32) {
        if let Some(param) = self.parameters_mut().iter_mut().find(|p| p.id == id) {
            param.set(value);
        }
    }
    
    /// Get parameter value by id
    fn get_parameter(&self, id: PortId) -> Option<f32> {
        self.parameters().iter().find(|p| p.id == id).map(|p| p.value)
    }
    
    /// Reset module state
    fn reset(&mut self);
    
    /// Module name for display
    fn name(&self) -> &'static str;
}

/// Connection between module ports
#[derive(Debug, Clone)]
pub struct Connection {
    pub source_module: usize,
    pub source_port: PortId,
    pub dest_module: usize,
    pub dest_port: PortId,
    pub amount: f32,
}

impl Connection {
    pub fn new(
        source_module: usize,
        source_port: PortId,
        dest_module: usize,
        dest_port: PortId,
        amount: f32,
    ) -> Self {
        Self {
            source_module,
            source_port,
            dest_module,
            dest_port,
            amount,
        }
    }
}

/// Module graph for routing
pub struct ModuleGraph {
    modules: Vec<Box<dyn Module>>,
    connections: Vec<Connection>,
    buffers: Vec<Vec<f32>>,
    port_values: HashMap<(usize, PortId), f32>,
}

impl ModuleGraph {
    pub fn new() -> Self {
        Self {
            modules: Vec::new(),
            connections: Vec::new(),
            buffers: Vec::new(),
            port_values: HashMap::new(),
        }
    }

    /// Add a module, returns its index
    pub fn add_module(&mut self, module: Box<dyn Module>) -> usize {
        let idx = self.modules.len();
        self.modules.push(module);
        self.buffers.push(Vec::new());
        idx
    }

    /// Add a connection between modules
    pub fn connect(
        &mut self,
        source_module: usize,
        source_port: PortId,
        dest_module: usize,
        dest_port: PortId,
        amount: f32,
    ) -> bool {
        if source_module >= self.modules.len() || dest_module >= self.modules.len() {
            return false;
        }
        self.connections.push(Connection::new(
            source_module,
            source_port,
            dest_module,
            dest_port,
            amount,
        ));
        true
    }

    /// Remove all connections
    pub fn clear_connections(&mut self) {
        self.connections.clear();
    }

    /// Get module count
    pub fn module_count(&self) -> usize {
        self.modules.len()
    }

    /// Get connection count
    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }

    /// Process the entire graph
    pub fn process(&mut self, output: &mut [f32], sample_rate: u32, bpm: f32) {
        let buffer_size = output.len();
        
        // Ensure buffers are sized
        for buf in &mut self.buffers {
            buf.resize(buffer_size, 0.0);
            buf.fill(0.0);
        }

        // Clear port values
        self.port_values.clear();

        // Process each module in order
        for (module_idx, module) in self.modules.iter_mut().enumerate() {
            // Gather input values from connections
            let mut inputs: HashMap<PortId, f32> = HashMap::new();
            
            // Set defaults
            for input in module.inputs() {
                inputs.insert(input.id, input.default);
            }
            
            // Apply connections
            for conn in &self.connections {
                if conn.dest_module == module_idx {
                    if let Some(&src_val) = self.port_values.get(&(conn.source_module, conn.source_port)) {
                        let entry = inputs.entry(conn.dest_port).or_insert(0.0);
                        *entry += src_val * conn.amount;
                    }
                }
            }

            let context = ModuleContext {
                sample_rate,
                buffer_size,
                inputs: &inputs,
                bpm,
            };

            // Process module
            module.process(&context, &mut self.buffers[module_idx]);

            // Store output values (use last sample as representative)
            for out_port in module.outputs() {
                let val = self.buffers[module_idx].last().copied().unwrap_or(0.0);
                self.port_values.insert((module_idx, out_port.id), val);
            }
        }

        // Copy last module's output to main output
        if let Some(last_buf) = self.buffers.last() {
            for (i, sample) in output.iter_mut().enumerate() {
                if i < last_buf.len() {
                    *sample = last_buf[i];
                }
            }
        }
    }

    /// Reset all modules
    pub fn reset(&mut self) {
        for module in &mut self.modules {
            module.reset();
        }
        self.port_values.clear();
    }
}

impl Default for ModuleGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Simple test module
    struct GainModule {
        gain: f32,
        inputs: Vec<InputPort>,
        outputs: Vec<OutputPort>,
        params: Vec<Parameter>,
    }

    impl GainModule {
        fn new(gain: f32) -> Self {
            Self {
                gain,
                inputs: vec![InputPort {
                    id: PortId(0),
                    name: "input",
                    default: 0.0,
                }],
                outputs: vec![OutputPort {
                    id: PortId(1),
                    name: "output",
                }],
                params: vec![Parameter::new(2, "gain", 0.0, 2.0, gain)],
            }
        }
    }

    impl Module for GainModule {
        fn process(&mut self, context: &ModuleContext, buffer: &mut [f32]) {
            let input_val = context.inputs.get(&PortId(0)).copied().unwrap_or(0.0);
            for sample in buffer.iter_mut() {
                *sample = input_val * self.gain;
            }
        }

        fn inputs(&self) -> &[InputPort] {
            &self.inputs
        }

        fn outputs(&self) -> &[OutputPort] {
            &self.outputs
        }

        fn parameters(&self) -> &[Parameter] {
            &self.params
        }

        fn parameters_mut(&mut self) -> &mut [Parameter] {
            &mut self.params
        }

        fn reset(&mut self) {}

        fn name(&self) -> &'static str {
            "Gain"
        }
    }

    #[test]
    fn test_port_id() {
        let p1 = PortId(1);
        let p2 = PortId(1);
        assert_eq!(p1, p2);
    }

    #[test]
    fn test_parameter() {
        let mut param = Parameter::new(0, "test", 0.0, 1.0, 0.5);
        assert_eq!(param.value, 0.5);
        param.set(0.8);
        assert_eq!(param.value, 0.8);
        param.set(2.0); // Clamped
        assert_eq!(param.value, 1.0);
    }

    #[test]
    fn test_connection() {
        let conn = Connection::new(0, PortId(1), 1, PortId(0), 0.5);
        assert_eq!(conn.source_module, 0);
        assert_eq!(conn.amount, 0.5);
    }

    #[test]
    fn test_module_graph_new() {
        let graph = ModuleGraph::new();
        assert_eq!(graph.module_count(), 0);
    }

    #[test]
    fn test_module_graph_add() {
        let mut graph = ModuleGraph::new();
        let idx = graph.add_module(Box::new(GainModule::new(1.0)));
        assert_eq!(idx, 0);
        assert_eq!(graph.module_count(), 1);
    }

    #[test]
    fn test_module_graph_connect() {
        let mut graph = ModuleGraph::new();
        graph.add_module(Box::new(GainModule::new(1.0)));
        graph.add_module(Box::new(GainModule::new(0.5)));
        assert!(graph.connect(0, PortId(1), 1, PortId(0), 1.0));
        assert_eq!(graph.connection_count(), 1);
    }

    #[test]
    fn test_module_graph_process() {
        let mut graph = ModuleGraph::new();
        graph.add_module(Box::new(GainModule::new(2.0)));
        
        let mut output = vec![0.0; 64];
        graph.process(&mut output, 44100, 120.0);
        // Should produce some output
        assert!(output.iter().all(|&s| s.is_finite()));
    }

    #[test]
    fn test_module_graph_reset() {
        let mut graph = ModuleGraph::new();
        graph.add_module(Box::new(GainModule::new(1.0)));
        graph.reset();
        assert_eq!(graph.module_count(), 1);
    }

    #[test]
    fn test_module_trait_set_parameter() {
        let mut module = GainModule::new(1.0);
        module.set_parameter(PortId(2), 0.7);
        assert_eq!(module.get_parameter(PortId(2)), Some(0.7));
    }
}
