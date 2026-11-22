-- unit.drone_operator@information: Drone operator (mesh.drone_operator_information)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.drone_operator_information','Drone operator');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.drone_operator_information';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('body','figure','mat.cloth',1,'','{"attack":7,"hold_right":55,"hold_left":55,"legs":"#5f5c4b"}'),
  ('helmet','dome','mat.khaki',0,'head','{"y":-0.11,"w":0.28,"h":0.15,"segments":6}'),
  ('headset','cylinder','mat.composite',0,'head','{"y":-0.12,"x":-0.13,"w":0.07,"h":0.26,"segments":4,"roll":90,"far":0}'),
  ('visor','box','mat.glass',0,'head','{"y":-0.02,"z":0.11,"w":0.2,"d":0.05,"h":0.05,"tint":"#506a88","far":0}'),
  ('controller','box','mat.composite',0,'right_hand','{"x":-0.1,"y":-0.06,"z":0.05,"w":0.3,"d":0.04,"h":0.2,"pitch":-55}'),
  ('screen','box','mat.glass',0,'controller','{"y":0.02,"z":0.025,"w":0.22,"d":0.01,"h":0.14,"tint":"#3f8fb0","far":0}'),
  ('controller_aerial','cylinder','mat.composite',0,'controller','{"x":0.12,"y":0.1,"w":0.02,"h":0.16,"segments":3,"far":0}'),
  ('pack','box','mat.khaki',0,'back','{"y":-0.32,"z":-0.08,"w":0.28,"d":0.16,"h":0.32}'),
  ('mast','cylinder','mat.composite',0,'pack','{"x":0.09,"y":0.3,"w":0.025,"h":0.6,"segments":3}'),
  ('mast_tip','dome','mat.cloth',1,'mast','{"y":0.6,"w":0.05,"h":0.04,"segments":4,"far":0}'),
  ('spare_drone','box','mat.composite',0,'pack','{"y":0.34,"w":0.16,"d":0.16,"h":0.05}'),
  ('spare_arms','box','mat.composite',0,'spare_drone','{"y":0.02,"w":0.36,"d":0.03,"h":0.02,"yaw":45,"far":0}'),
  ('spare_arms_cross','box','mat.composite',0,'spare_drone','{"y":0.02,"w":0.36,"d":0.03,"h":0.02,"yaw":-45,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.drone_operator_information'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.drone_operator'), NULL, (SELECT id FROM def WHERE key='age.information'), (SELECT id FROM def WHERE key='mesh.drone_operator_information'), (SELECT id FROM def WHERE key='mat.cloth'), 100);
