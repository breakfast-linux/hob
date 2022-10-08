use crate::definition::actions::{
    Action, ActionPlaybook, BinAction, CcAction, DirAction, LinkAction, ManAction, RmAction, Stage,
};
use crate::definition::parsing::{GetNodes, HobParseError, ParseNode, ProxyMap};
use crate::parse_string_list_into;
use kdl::KdlNode;

impl ParseNode for ActionPlaybook {
    fn parse_node_with_errors(input: &KdlNode) -> (Option<Self>, Vec<HobParseError>)
    where
        Self: Sized,
    {
        let st = if let Some(st) = Stage::from_str(input.name().value()) {
            st
        } else {
            return (
                None,
                vec![HobParseError {
                    span: *input.name().span(),
                    label: None,
                    help: None,
                    kind: "unknown stage",
                }],
            );
        };

        let mut error = vec![];

        let mut actions = vec![];
        for node in input.nodes() {
            let (act, err) = Action::parse_node_with_errors(node);
            error.extend(err);

            if let Some(act) = act {
                actions.push(act);
            }
        }

        (Some(ActionPlaybook { stage: st, actions }), error)
    }
}

impl ParseNode for Action {
    fn parse_node_with_errors(input: &KdlNode) -> (Option<Self>, Vec<HobParseError>)
    where
        Self: Sized,
    {
        match input.name().value() {
            ".default" => (Action::Default.into(), vec![]),
            "make" => (Action::Make.into(), vec![]),
            "make-install" => (Action::MakeInstall.into(), vec![]),
            "cc" => CcAction::parse_node_with_errors(input).map(Action::Cc),
            "bin" => BinAction::parse_node_with_errors(input).map(Action::Bin),
            "man" => ManAction::parse_node_with_errors(input).map(Action::Man),
            "rm" => RmAction::parse_node_with_errors(input).map(Action::Rm),
            "dir" => DirAction::parse_node_with_errors(input).map(Action::Dir),
            "link" => LinkAction::parse_node_with_errors(input).map(Action::Link),
            _ => (
                None,
                vec![HobParseError {
                    span: *input.span(),
                    label: None,
                    help: None,
                    kind: "unknown action",
                }],
            ),
        }
    }
}

impl ParseNode for CcAction {
    fn parse_node_with_errors(input: &KdlNode) -> (Option<Self>, Vec<HobParseError>)
    where
        Self: Sized,
    {
        let mut errors = vec![];
        let mut output = None;
        let mut inputs = vec![];

        for entry in input.entries() {
            if let Some(v) = entry.name() {
                if v.value() == "output" {
                    if output.is_some() {
                        errors.push(HobParseError {
                            span: *entry.span(),
                            label: Some("second output definition"),
                            help: None,
                            kind: "output is defined multiple times for cc",
                        });
                    }

                    if let Some(v) = entry.value().as_string() {
                        output = Some(v);
                    } else {
                        errors.push(HobParseError {
                            span: *entry.span(),
                            label: None,
                            help: None,
                            kind: "output value should be a string",
                        });
                    }
                }
            } else if let Some(v) = entry.value().as_string() {
                inputs.push(v.to_string());
            } else {
                errors.push(HobParseError {
                    span: *entry.span(),
                    label: None,
                    help: None,
                    kind: "input value should be a string",
                });
            }
        }

        (
            output
                .filter(|_| !inputs.is_empty())
                .map(|output| CcAction {
                    input: inputs,
                    output: output.to_string(),
                }),
            errors,
        )
    }
}

impl ParseNode for BinAction {
    fn parse_node_with_errors(input: &KdlNode) -> (Option<Self>, Vec<HobParseError>)
    where
        Self: Sized,
    {
        let mut args = vec![];
        let mut errors = vec![];
        parse_string_list_into!(input, args, errors, "bin arguments");

        (Some(BinAction { binaries: args }), errors)
    }
}

impl ParseNode for ManAction {
    fn parse_node_with_errors(input: &KdlNode) -> (Option<Self>, Vec<HobParseError>)
    where
        Self: Sized,
    {
        let mut args = vec![];
        let mut errors = vec![];
        parse_string_list_into!(input, args, errors, "bin arguments");

        (Some(ManAction { man_files: args }), errors)
    }
}

impl ParseNode for DirAction {
    fn parse_node_with_errors(input: &KdlNode) -> (Option<Self>, Vec<HobParseError>)
    where
        Self: Sized,
    {
        let mut args = vec![];
        let mut errors = vec![];
        parse_string_list_into!(input, args, errors, "bin arguments");

        (Some(DirAction { targets: args }), errors)
    }
}

impl ParseNode for RmAction {
    fn parse_node_with_errors(input: &KdlNode) -> (Option<Self>, Vec<HobParseError>)
    where
        Self: Sized,
    {
        let mut args = vec![];
        let mut errors = vec![];
        parse_string_list_into!(input, args, errors, "bin arguments");

        (Some(RmAction { targets: args }), errors)
    }
}

impl ParseNode for LinkAction {
    fn parse_node_with_errors(input: &KdlNode) -> (Option<Self>, Vec<HobParseError>)
    where
        Self: Sized,
    {
        let mut args = vec![];
        let mut errors = vec![];
        parse_string_list_into!(input, args, errors, "bin arguments");

        if args.len() < 2 {
            errors.push(HobParseError {
                span: *input.span(),
                label: None,
                help: None,
                kind: "link needs at least 2 arguments, source and target",
            });
            (None, errors)
        } else {
            (
                Some(LinkAction {
                    target: args.pop().unwrap(),
                    source: args,
                }),
                errors,
            )
        }
    }
}
